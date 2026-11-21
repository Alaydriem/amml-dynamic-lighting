use std::sync::Arc;

use amml_api::amml_abi::{ClientTick, WorldLeft};
use amml_api::{ApiError, Ctx, HookOrder, Listener};

use crate::services::LightRuntime;

/// The loader listeners of the light; each hands its event to the runtime. Dropping them removes them.
pub struct LightListeners {
    listeners: Vec<Listener>,
    runtime: Arc<LightRuntime>,
}

impl LightListeners {
    pub fn start(ctx: &Ctx, runtime: Arc<LightRuntime>) -> Result<Self, ApiError> {
        let events = ctx.events();
        let ticking = Arc::clone(&runtime);
        let tick = events.listen::<ClientTick>(HookOrder::NORMAL, move |_, at| ticking.tick(at.client()))?;
        let leaving = Arc::clone(&runtime);
        let leave = events.listen::<WorldLeft>(HookOrder::NORMAL, move |_, _| leaving.world_left())?;
        Ok(Self { listeners: vec![tick, leave], runtime })
    }

    /// The listeners go first, so no listener runs while the runtime lets go of the link.
    pub fn stop(self) {
        let Self { listeners, runtime } = self;
        drop(listeners);
        drop(runtime.detach());
    }
}
