use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SymbolPlacementConfig {
    pub(crate) unit: u32,
    pub(crate) body_style: Option<u32>,
    pub(crate) pin_alternates: BTreeMap<String, String>,
}

impl Default for SymbolPlacementConfig {
    fn default() -> Self {
        Self {
            unit: 1,
            body_style: None,
            pin_alternates: BTreeMap::new(),
        }
    }
}

impl SymbolPlacementConfig {
    pub(crate) fn unit_option(&self) -> Option<u32> {
        Some(self.unit.max(1))
    }

    pub(crate) fn selected_body_style(&self) -> u32 {
        self.body_style.unwrap_or(1)
    }
}
