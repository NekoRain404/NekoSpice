use osl_kicad::{KicadPoint, KicadSize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SchematicTool {
    Select,
    Wire,
    Bus,
    BusEntry,
    Label,
    GlobalLabel,
    HierarchicalLabel,
    Text,
    Junction,
    NoConnect,
}

impl SchematicTool {
    pub(super) const ALL: [Self; 10] = [
        Self::Select,
        Self::Wire,
        Self::Bus,
        Self::BusEntry,
        Self::Label,
        Self::GlobalLabel,
        Self::HierarchicalLabel,
        Self::Text,
        Self::Junction,
        Self::NoConnect,
    ];

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Select => "Select",
            Self::Wire => "Wire",
            Self::Bus => "Bus",
            Self::BusEntry => "Bus Entry",
            Self::Label => "Label",
            Self::GlobalLabel => "Global",
            Self::HierarchicalLabel => "Hier Label",
            Self::Text => "Text",
            Self::Junction => "Junction",
            Self::NoConnect => "No Connect",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SchematicToolState {
    pub(super) active: SchematicTool,
    pub(super) label_text: String,
    pub(super) text_item: String,
    pub(super) pending_wire_start: Option<KicadPoint>,
    pub(super) pending_bus_start: Option<KicadPoint>,
    pub(super) bus_entry_size: KicadSize,
}

impl Default for SchematicToolState {
    fn default() -> Self {
        Self {
            active: SchematicTool::Select,
            label_text: "net".to_string(),
            text_item: ".save v(out)".to_string(),
            pending_wire_start: None,
            pending_bus_start: None,
            bus_entry_size: KicadSize {
                width: 2.54,
                height: -2.54,
            },
        }
    }
}

impl SchematicToolState {
    pub(crate) fn clear_pending(&mut self) {
        self.pending_wire_start = None;
        self.pending_bus_start = None;
    }

    pub(super) fn set_active(&mut self, tool: SchematicTool) {
        if self.active != tool {
            self.active = tool;
            self.clear_pending();
        }
    }

    pub(super) fn has_pending(&self) -> bool {
        self.pending_wire_start.is_some() || self.pending_bus_start.is_some()
    }
}
