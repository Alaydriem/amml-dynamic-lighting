use std::sync::{Mutex, MutexGuard, PoisonError};

use amm_client::link::{LinkState, ManagerLink};
use amm_core::structs::event::{KeybindPhase, ModEvent};
use amml_api::{Client, Hand, ItemView, LightFalloff, LightSource, Lighting, Logger};

use crate::lighting_settings::{LightingConfig, LightingSettings};
use crate::services::LightPicker;
use crate::structs::HeldLight;

/// What the listeners share: the manager link, the settings, the toggle and the last light given to the loader.
/// Only the game thread (the client tick, the world leave) and enable or disable take the lock.
pub struct LightRuntime {
    log: Logger,
    lighting: Lighting,
    state: Mutex<LightState>,
}

struct LightState {
    link: Option<ManagerLink>,
    configured: bool,
    settings: LightingConfig,
    toggled_on: bool,
    // The light the loader holds for this mod.
    sent: Option<LightSource>,
    // The level, falloff and block of the last `light source set` line.
    logged: Option<(u8, LightFalloff, bool)>,
    names_logged: bool,
    refusal_logged: bool,
}

impl LightRuntime {
    // The toggle key is action 0 of the descriptor.
    const TOGGLE_ACTION: u16 = 0;

    pub fn new(log: Logger, lighting: Lighting) -> Self {
        let state = LightState {
            link: None,
            configured: false,
            settings: LightingConfig::default(),
            toggled_on: true,
            sent: None,
            logged: None,
            names_logged: false,
            refusal_logged: false,
        };
        Self { log, lighting, state: Mutex::new(state) }
    }

    fn lock(&self) -> MutexGuard<'_, LightState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Enable: the light works with this link (or with the defaults and no toggle key without one).
    pub fn attach(&self, link: Option<ManagerLink>) {
        let mut state = self.lock();
        state.link = link;
        state.configured = false;
    }

    /// Disable: the link closes. The loader clears this mod's light itself.
    pub fn detach(&self) -> Option<ManagerLink> {
        let mut state = self.lock();
        state.sent = None;
        state.logged = None;
        state.link.take()
    }

    /// Before a world leave: the loader drops every light, so the next world starts with none sent.
    pub fn world_left(&self) {
        let mut state = self.lock();
        state.sent = None;
        state.logged = None;
    }

    /// Once per client tick on the game thread: reads the manager's events, picks the light and gives it to the
    /// loader when it differs from the one the loader holds.
    pub fn tick(&self, client: Option<Client<'_>>) {
        let mut state = self.lock();
        self.drain(&mut state);
        let wanted = self.wanted(&mut state, client.as_ref());
        if wanted == state.sent {
            return;
        }
        match self.lighting.set_light_source(wanted) {
            Ok(()) => {
                state.sent = wanted;
                state.refusal_logged = false;
                self.log_change(&mut state, wanted);
            }
            Err(error) => {
                if !state.refusal_logged {
                    state.refusal_logged = true;
                    self.log.warn("light source refused", &[("error", &error.to_string())]);
                }
            }
        }
    }

    fn wanted(&self, state: &mut LightState, client: Option<&Client<'_>>) -> Option<LightSource> {
        if !state.settings.enabled || !state.toggled_on {
            return None;
        }
        let player = client?.local_player()?;
        let main = player.held_item(Hand::Main).map(|item| self.held(state, &item));
        let off = player.held_item(Hand::Off).map(|item| self.held(state, &item));
        // The light sits where the game kept the old light: the player's own position (the eyes).
        let position = player.eye_position().or_else(|| player.position());
        LightPicker::pick(main.as_ref(), off.as_ref(), &state.settings, state.toggled_on, position)
    }

    // One hand. The name is read only for an item without block light, the case of the exceptions.
    fn held(&self, state: &mut LightState, item: &ItemView<'_>) -> HeldLight {
        let emission = item.light_emission();
        let block = item.block();
        let name = if emission.unwrap_or(0) == 0 { item.type_name() } else { None };
        if !state.names_logged {
            let named = name.is_some() || item.type_name().is_some();
            state.names_logged = true;
            self.log.info(if named { "item names: on" } else { "item names: off" }, &[]);
        }
        HeldLight { emission, block, name }
    }

    fn log_change(&self, state: &mut LightState, sent: Option<LightSource>) {
        let Some(light) = sent else {
            state.logged = None;
            self.log.info("light source cleared", &[]);
            return;
        };
        let key = (light.level, light.falloff, light.block.is_some());
        if state.logged == Some(key) {
            return;
        }
        state.logged = Some(key);
        let falloff = match light.falloff {
            LightFalloff::Round => "round",
            LightFalloff::Taxicab => "taxicab",
        };
        let block = if light.block.is_some() { "yes" } else { "no" };
        self.log
            .info("light source set", &[("level", &light.level.to_string()), ("falloff", falloff), ("block", block)]);
    }

    fn drain(&self, state: &mut LightState) {
        let Some(link) = state.link.as_mut() else { return };
        let connected = link.state() == LinkState::Connected;
        let mut settings = None;
        if connected && !state.configured {
            state.configured = true;
            settings = Some(Self::read_settings(link, &self.log));
            self.log.info("manager attached", &[]);
        }
        let events: Vec<ModEvent> = link.events().collect();
        for event in events {
            match event {
                ModEvent::Keybind { action, phase: KeybindPhase::Pressed } if action == Self::TOGGLE_ACTION => {
                    state.toggled_on = !state.toggled_on;
                    self.log.info(if state.toggled_on { "toggled on" } else { "toggled off" }, &[]);
                }
                ModEvent::SettingsChanged { .. } => settings = Some(Self::read_settings(link, &self.log)),
                ModEvent::Disabled => state.toggled_on = false,
                ModEvent::Enabled => state.toggled_on = true,
                _ => {}
            }
        }
        if let Some(settings) = settings {
            state.settings = settings;
            self.log.info(
                "settings",
                &[
                    ("enabled", &settings.enabled.to_string()),
                    ("round", &settings.round.to_string()),
                    ("level_override", &settings.level_override.to_string()),
                ],
            );
        }
    }

    fn read_settings(link: &ManagerLink, log: &Logger) -> LightingConfig {
        match link.config::<LightingSettings>() {
            Ok(settings) => settings.into_config(),
            Err(error) => {
                log.warn("config could not be read; using the defaults", &[("error", &error.to_string())]);
                LightingConfig::default()
            }
        }
    }
}
