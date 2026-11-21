/// The descriptor the manager registers the mod from: the WorldDraw claim, the toggle keybind and the settings.
pub struct LightDescriptor;

impl LightDescriptor {
    pub const TEXT: &'static str = include_str!("../../descriptor.hcl");
}
