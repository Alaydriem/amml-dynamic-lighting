use std::sync::{Arc, Mutex, PoisonError};

use amm_client::link::ManagerLink;
use amml_api::amml_abi::{
    ClientTick, InjectedInputReleased, InputLeaseLost, InputLeaseResumed, InputLeaseSuspended, ModsLoaded,
    ProviderGone, ProviderReady, ScreenshotTaken, ScreenshotWritten, WorldLeft,
};
use amml_api::{AmmlMod, ApiError, Area, Ctx, DisableCtx, Uses};

use crate::services::{LightDescriptor, LightListeners, LightRuntime};

// What the enabled mod owns. `disable` takes it, which drops the listeners and closes the link.
static RUNNING: Mutex<Option<LightListeners>> = Mutex::new(None);

/// The AMML entry point: enable starts the manager link and the listeners, disable removes them. The loader clears
/// the mod's light at disable.
pub struct DynamicLightingMod;

impl AmmlMod for DynamicLightingMod {
    const NAMESPACE: &'static str = "alaydriem";
    const NAME: &'static str = "dynamic-lighting";
    const USES: Uses = amml_api::amml_uses! {
        feature "light" {
            required ClientTick;
            required WorldLeft;
            required ProviderReady;
            required ProviderGone;
            required area Area::CLIENT_READ;
            required area Area::ACTOR_READ.at(2);
            required area Area::INVENTORY_READ.at(2);
            required area Area::LIGHTING_TERRAIN;
            // The manager link listens to these when the loader serves them
            optional ModsLoaded;
            optional InputLeaseSuspended;
            optional InputLeaseResumed;
            optional InputLeaseLost;
            optional InjectedInputReleased;
            optional ScreenshotTaken;
            optional ScreenshotWritten;
            optional area Area::INPUT_LEASE;
        }
        feature "entity light" {
            required area Area::LIGHTING_ENTITY;
        }
        feature "vibrant colour" {
            required area Area::LIGHTING_VIBRANT;
        }
    };

    fn enable(ctx: &Ctx) -> Result<(), ApiError> {
        let log = ctx.log();
        let link = match ManagerLink::start_without_frame(ctx, LightDescriptor::TEXT) {
            Ok(link) => Some(link),
            Err(error) => {
                log.warn("mod manager not reached; defaults in use, no toggle key", &[("error", &error.to_string())]);
                None
            }
        };
        let runtime = Arc::new(LightRuntime::new(log, ctx.lighting()));
        runtime.attach(link);
        let listeners = LightListeners::start(ctx, runtime)?;
        *RUNNING.lock().unwrap_or_else(PoisonError::into_inner) = Some(listeners);
        let entity = if ctx.supports(Area::LIGHTING_ENTITY) { "entity light: on" } else { "entity light: off" };
        let vibrant = if ctx.supports(Area::LIGHTING_VIBRANT) { "vibrant colour: on" } else { "vibrant colour: off" };
        log.info(entity, &[]);
        log.info(vibrant, &[]);
        log.info("enabled", &[]);
        Ok(())
    }

    fn disable(_ctx: &Ctx, _how: &DisableCtx) {
        let running = RUNNING.lock().unwrap_or_else(PoisonError::into_inner).take();
        if let Some(listeners) = running {
            listeners.stop();
        }
    }
}
