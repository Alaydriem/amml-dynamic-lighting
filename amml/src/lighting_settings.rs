use serde::Deserialize;

/// The `lighting` block of config.hcl.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct LightingConfig {
    pub enabled: bool,
    pub round: bool,
    /// 1 to 15: held light items light at this level instead of their own; 0 uses the item level.
    pub level_override: u8,
}

impl Default for LightingConfig {
    fn default() -> Self {
        Self { enabled: true, round: true, level_override: 0 }
    }
}

/// The whole config.hcl document; the `keys` block belongs to the manager.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct LightingSettings {
    pub lighting: LightingConfig,
}

impl LightingSettings {
    pub fn into_config(self) -> LightingConfig {
        LightingConfig { level_override: self.lighting.level_override.min(15), ..self.lighting }
    }
}
