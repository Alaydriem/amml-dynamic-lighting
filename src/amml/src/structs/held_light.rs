use amml_api::BlockHandle;

/// What one hand holds, as far as the light needs it: the light the item's block gives (`None` for an item that
/// places no block), that block, and the item's full name (`None` when the build cannot read it).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeldLight {
    pub emission: Option<u8>,
    pub block: Option<BlockHandle>,
    pub name: Option<String>,
}
