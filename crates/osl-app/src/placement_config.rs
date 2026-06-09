#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SymbolPlacementConfig {
    pub(crate) unit: u32,
    pub(crate) body_style: Option<u32>,
}

impl Default for SymbolPlacementConfig {
    fn default() -> Self {
        Self {
            unit: 1,
            body_style: None,
        }
    }
}

impl SymbolPlacementConfig {
    pub(crate) fn unit_option(self) -> Option<u32> {
        Some(self.unit.max(1))
    }
}
