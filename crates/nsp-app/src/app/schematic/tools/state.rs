//! Schematic tool state — definitions for available drawing tools and their runtime state.

use nsp_schema::{NspPoint, NspSize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// 原理图绘图工具枚举。
///
/// 表示当前激活的绘图工具：选择、导线、总线、标签、
/// 网络标签、电源端口、电阻、电容等。
pub(crate) enum SchematicTool {
    Select,
    Wire,
    Bus,
    BusEntry,
    Label,
    GlobalLabel,
    HierarchicalLabel,
    Sheet,
    Text,
    Junction,
    NoConnect,
}

impl SchematicTool {
    pub(crate) const ALL: [Self; 11] = [
        Self::Select,
        Self::Wire,
        Self::Bus,
        Self::BusEntry,
        Self::Label,
        Self::GlobalLabel,
        Self::HierarchicalLabel,
        Self::Sheet,
        Self::Text,
        Self::Junction,
        Self::NoConnect,
    ];

    /// label。
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Select => "Select",
            Self::Wire => "Wire",
            Self::Bus => "Bus",
            Self::BusEntry => "Bus Entry",
            Self::Label => "Label",
            Self::GlobalLabel => "Global",
            Self::HierarchicalLabel => "Hier Label",
            Self::Sheet => "Sheet",
            Self::Text => "Text",
            Self::Junction => "Junction",
            Self::NoConnect => "No Connect",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
/// 绘图工具状态。跟踪当前活跃工具和工具切换的临时状态。
pub(crate) struct SchematicToolState {
    pub(crate) active: SchematicTool,
    pub(crate) label_text: String,
    pub(crate) text_item: String,
    pub(crate) sheet_name: String,
    pub(crate) sheet_file: String,
    pub(crate) sheet_size: NspSize,
    pub(crate) sheet_pin_names: [String; 2],
    pub(crate) pending_wire_start: Option<NspPoint>,
    pub(crate) pending_bus_start: Option<NspPoint>,
    pub(crate) bus_entry_size: NspSize,
}

impl Default for SchematicToolState {
    fn default() -> Self {
        Self {
            active: SchematicTool::Select,
            label_text: "net".to_string(),
            text_item: ".save v(out)".to_string(),
            sheet_name: "sheet".to_string(),
            sheet_file: "sheet.nsp_sch".to_string(),
            sheet_size: NspSize {
                width: 25.4,
                height: 12.7,
            },
            sheet_pin_names: ["in".to_string(), "out".to_string()],
            pending_wire_start: None,
            pending_bus_start: None,
            bus_entry_size: NspSize {
                width: 2.54,
                height: -2.54,
            },
        }
    }
}

impl SchematicToolState {
    /// clear pending。
    pub(crate) fn clear_pending(&mut self) {
        self.pending_wire_start = None;
        self.pending_bus_start = None;
    }

    /// set active。
    pub(crate) fn set_active(&mut self, tool: SchematicTool) {
        if self.active != tool {
            self.active = tool;
            self.clear_pending();
        }
    }

    /// has pending。
    pub(crate) fn has_pending(&self) -> bool {
        self.pending_wire_start.is_some() || self.pending_bus_start.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn switching_schematic_tool_clears_pending_wire_start() {
        let mut state = SchematicToolState::default();
        state.set_active(SchematicTool::Wire);
        state.pending_wire_start = Some(NspPoint { x: 1.0, y: 2.0 });

        state.set_active(SchematicTool::Label);

        assert_eq!(state.active, SchematicTool::Label);
        assert!(state.pending_wire_start.is_none());
    }
}
