use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

use amm_client::link::{ApiTable, ManagerLink};
use amm_core::structs::event::{KeybindPhase, ModEvent};
use amml_game::{ClientInstance, GameEntries};
use amml_sdk::{HookHandle, Logger};

use crate::light_source::LightSource;
use crate::lighting_settings::{LightingConfig, LightingSettings};
use crate::player_inventory::{HeldItem, PlayerInventory};
use crate::remesh_plan::{HeldLight, RemeshPlan};
use crate::remesher::Remesher;

const DESCRIPTOR: &str = include_str!("../descriptor.hcl");

// link states
const LINK_OFF: u8 = 0;
const LINK_WAITING: u8 = 1;
const LINK_ATTACHED: u8 = 2;

/// Game-thread state between the update tick and the render call.
struct FrameState {
    settings: LightingConfig,
    toggled_on: bool,
    published: Option<HeldLight>,
    /// The `Block const*` of the published light (0: none, or an item that places no block).
    published_block: usize,
    pending: Option<(Option<HeldLight>, Option<HeldLight>)>,
}

/// The mod's process-wide state.
pub struct DynamicLightingRuntime {
    logger: Logger,
    config_dir: PathBuf,
    entries: OnceLock<GameEntries>,
    hooks: Mutex<Vec<HookHandle>>,
    link: Mutex<Option<ManagerLink>>,
    link_state: AtomicU8,
    client: AtomicUsize,
    level_renderer: AtomicUsize,
    remesh_refused: AtomicBool,
    frame: Mutex<FrameState>,
}

static RUNTIME: OnceLock<DynamicLightingRuntime> = OnceLock::new();

impl DynamicLightingRuntime {
    pub fn install(logger: Logger, config_dir: PathBuf) {
        let _ = RUNTIME.set(Self {
            logger,
            config_dir,
            entries: OnceLock::new(),
            hooks: Mutex::new(Vec::new()),
            link: Mutex::new(None),
            link_state: AtomicU8::new(LINK_OFF),
            client: AtomicUsize::new(0),
            level_renderer: AtomicUsize::new(0),
            remesh_refused: AtomicBool::new(false),
            frame: Mutex::new(FrameState {
                settings: LightingConfig::default(),
                toggled_on: true,
                published: None,
                published_block: 0,
                pending: None,
            }),
        });
    }

    pub fn get() -> Option<&'static Self> {
        RUNTIME.get()
    }

    pub fn logger(&self) -> &Logger {
        &self.logger
    }

    pub fn set_entries(&self, entries: GameEntries) {
        let _ = self.entries.set(entries);
    }

    pub fn keep_hook(&self, hook: HookHandle) {
        if let Ok(mut hooks) = self.hooks.lock() {
            hooks.push(hook);
        }
    }

    pub fn take_hooks(&self) -> Vec<HookHandle> {
        self.hooks.lock().map(|mut hooks| std::mem::take(&mut *hooks)).unwrap_or_default()
    }

    pub fn enable_link(&self) {
        self.link_state.store(LINK_WAITING, Ordering::SeqCst);
    }

    /// Disable: the light goes out and the manager link is dropped.
    pub fn detach(&self) {
        self.link_state.store(LINK_OFF, Ordering::SeqCst);
        LightSource::get().publish(None, 0, true);
        if let Ok(mut link) = self.link.lock() {
            *link = None;
        }
    }

    // ---- ClientInstance::update, game thread, after the original ----

    pub fn tick(&self, client: usize) {
        self.client.store(client, Ordering::Release);
        self.attach_if_waiting();
        let Some(entries) = self.entries.get() else { return };
        let Ok(mut frame) = self.frame.lock() else { return };
        self.drain_events(&mut frame);

        // SAFETY: the game passes its live ClientInstance to update
        #[allow(unsafe_code)]
        let client = unsafe { ClientInstance::from_raw(client as *mut c_void, entries) };
        let (wanted, block) = client
            .and_then(|client| Self::wanted_light(&client, entries, &frame))
            .map_or((None, 0), |(light, block)| (Some(light), block));
        if wanted != frame.published || block != frame.published_block {
            LightSource::get().publish(wanted, block, frame.settings.round);
            let old = frame.published;
            frame.published = wanted;
            frame.published_block = block;
            // join with a request the render call has not taken yet, so no lit area is left behind
            let first_old = frame.pending.map_or(old, |(first, _)| first);
            frame.pending = Some((first_old, wanted));
        }
    }

    fn wanted_light(client: &ClientInstance, entries: &GameEntries, frame: &FrameState) -> Option<(HeldLight, usize)> {
        if !frame.settings.enabled || !frame.toggled_on {
            return None;
        }
        let player = client.local_player()?;
        let pose = player.pose()?;
        // SAFETY: the live local player, on the game thread
        #[allow(unsafe_code)]
        let main = unsafe { PlayerInventory::main_hand(player.as_ptr(), entries) };
        // SAFETY: as above
        #[allow(unsafe_code)]
        let off = unsafe { PlayerInventory::offhand(player.as_ptr(), entries) };
        // the brighter of the two hands gives the light
        let level_of = |held: &Option<HeldItem>| held.as_ref().map_or(0, |held| held.level);
        let brighter = if level_of(&off) > level_of(&main) { &off } else { &main };
        let item_level = level_of(brighter);
        let block = brighter.as_ref().map_or(0, |held| held.block);
        // the override changes the level of a held light item only; other items give no light
        let level = if item_level > 0 && frame.settings.level_override > 0 {
            frame.settings.level_override
        } else {
            item_level
        };
        (level > 0).then(|| (HeldLight::at(pose.pos, level), block))
    }

    fn attach_if_waiting(&self) {
        if self.link_state.load(Ordering::SeqCst) != LINK_WAITING {
            return;
        }
        let result =
            ApiTable::resolve().and_then(|table| ManagerLink::without_frame(table, DESCRIPTOR, &self.config_dir));
        match result {
            Ok(link) => {
                if let Ok(mut frame) = self.frame.lock() {
                    frame.settings = Self::read_settings(&link);
                    self.logger.info(&format!("manager attached; settings {:?}", frame.settings));
                }
                if let Ok(mut slot) = self.link.lock() {
                    *slot = Some(link);
                }
                self.link_state.store(LINK_ATTACHED, Ordering::SeqCst);
            }
            Err(error) => {
                self.logger.warn(&format!("mod manager not reached ({error}); defaults in use, no toggle key"));
                self.link_state.store(LINK_OFF, Ordering::SeqCst);
            }
        }
    }

    fn read_settings(link: &ManagerLink) -> LightingConfig {
        link.config::<LightingSettings>().map(LightingSettings::into_config).unwrap_or_default()
    }

    fn drain_events(&self, frame: &mut FrameState) {
        let Ok(mut slot) = self.link.lock() else { return };
        let Some(link) = slot.as_mut() else { return };
        let events: Vec<ModEvent> = link.events().collect();
        for event in events {
            match event {
                ModEvent::Keybind { action: 0, phase: KeybindPhase::Pressed } => {
                    frame.toggled_on = !frame.toggled_on;
                    self.logger.info(&format!("toggled {}", if frame.toggled_on { "on" } else { "off" }));
                }
                ModEvent::SettingsChanged { .. } => {
                    frame.settings = Self::read_settings(link);
                    // a change of shape needs a rebuild even when the light did not move
                    let current = frame.published;
                    frame.pending = Some((current, current));
                    LightSource::get().publish(current, frame.published_block, frame.settings.round);
                    self.logger.info(&format!("settings {:?}", frame.settings));
                }
                ModEvent::Disabled => frame.toggled_on = false,
                ModEvent::Enabled => frame.toggled_on = true,
                _ => {}
            }
        }
    }

    // ---- LevelRenderer::renderLevel, before the original ----

    pub fn render(&self, level_renderer: usize) {
        self.level_renderer.store(level_renderer, Ordering::Release);
        let Some(entries) = self.entries.get() else { return };
        let Ok(mut frame) = self.frame.lock() else { return };
        if let Some((old, new)) = frame.pending.take() {
            self.remesh(level_renderer, entries, old, new);
        }
    }

    fn remesh(&self, level_renderer: usize, entries: &GameEntries, old: Option<HeldLight>, new: Option<HeldLight>) {
        if self.remesh_refused.load(Ordering::Relaxed) {
            return;
        }
        let boxes = RemeshPlan::boxes(old, new);
        // SAFETY: the LevelRenderer renderLevel was called with, on the game thread, before this frame's update
        #[allow(unsafe_code)]
        let result = unsafe { Remesher::mark(level_renderer as *mut c_void, entries, &boxes) };
        if let Err(reason) = result {
            self.logger.warn(&format!("rebuild refused: {reason}"));
            self.remesh_refused.store(true, Ordering::Relaxed);
        }
    }
}
