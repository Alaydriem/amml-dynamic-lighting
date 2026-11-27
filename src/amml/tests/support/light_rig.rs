use alaydriem_dynamic_lighting::DynamicLightingMod;
use amm_client::test_support::FakeTable;
use amm_core::structs::event::{KeybindPhase, ModEvent};
use amml_api::amml_abi::{ClientTick, WorldLeft};
use amml_api::testing::{FakeClient, FakeGame, FakeInventory, FakeItem};
use amml_api::{Area, Screen, Vec3};

pub const TORCH: FakeItem =
    FakeItem { type_name: "minecraft:torch", count: 1, damage: 0, max_damage: 0, ..FakeItem::EMPTY };
pub const GLOWSTONE: FakeItem =
    FakeItem { type_name: "minecraft:glowstone", count: 1, damage: 0, max_damage: 0, ..FakeItem::EMPTY };
pub const LAVA: FakeItem =
    FakeItem { type_name: "minecraft:lava_bucket", count: 1, damage: 0, max_damage: 0, ..FakeItem::EMPTY };
pub const EYES: Vec3 = Vec3 { x: 0.5, y: 65.62, z: 0.5 };

/// The mod enabled on a fake game that serves every row of its `USES`, with the mod manager provided, in a world
/// with a local player at `EYES`.
pub struct LightRig {
    pub game: FakeGame,
}

impl LightRig {
    pub fn new(config: &str) -> Self {
        let game = FakeTable::game();
        game.offer::<ClientTick>();
        game.offer::<WorldLeft>();
        for area in [
            Area::CLIENT_READ,
            Area::ACTOR_READ.at(2),
            Area::INVENTORY_READ.at(2),
            Area::LIGHTING_TERRAIN,
            Area::LIGHTING_ENTITY,
            Area::LIGHTING_VIBRANT,
        ] {
            game.offer_area(area);
        }
        FakeTable::provide(&game.ctx()).unwrap();
        FakeTable::set_config(config);
        game.set_item_block("minecraft:torch", 14);
        game.set_item_block("minecraft:glowstone", 15);
        game.load::<DynamicLightingMod>().unwrap();
        game.enable::<DynamicLightingMod>().unwrap();
        game.open_gate();
        game.set_epoch(1);
        game.set_client(FakeClient {
            local_player: true,
            world_active: true,
            top_screen: Screen::Other,
            eye_position: Some(EYES),
            ..FakeClient::default()
        });
        Self { game }
    }

    pub fn hold(&self, main: Option<FakeItem>, off: Option<FakeItem>) {
        self.game.set_inventory(FakeInventory { held: main, selected_slot: Some(0), ..FakeInventory::default() });
        self.game.set_off_hand(off);
    }

    pub fn move_to(&self, eyes: Vec3) {
        self.game.set_client(FakeClient {
            local_player: true,
            world_active: true,
            top_screen: Screen::Other,
            eye_position: Some(eyes),
            ..FakeClient::default()
        });
    }

    pub fn tick(&self) {
        self.game.raise_with_client(&mut ClientTick { header: FakeGame::header::<ClientTick>() });
    }

    pub fn press_toggle(&self) {
        FakeTable::push_event(ModEvent::Keybind { action: 0, phase: KeybindPhase::Pressed });
        self.tick();
    }

    pub fn world_left(&self) {
        self.game.raise(&mut WorldLeft { header: FakeGame::header::<WorldLeft>(), epoch: 1 });
    }

    pub fn logged(&self, line: &str) -> bool {
        self.game.logs().iter().any(|logged| logged == line)
    }
}
