//! 撤销/重做历史栈。基于快照的编辑历史管理。
//!
use osl_kicad::KicadSchematic;

#[derive(Debug, Clone, Default)]
pub(crate) struct EditHistory {
    /// Snapshots to undo back to (most recent last).
    undo_stack: Vec<KicadSchematic>,
    /// Snapshots to redo forward to (most recent last).
    redo_stack: Vec<KicadSchematic>,
}

impl EditHistory {
    /// Save a snapshot of the current schematic before an edit is applied.
    ///
    /// Callers should invoke this *before* mutating the document.  After the
    /// mutation succeeds, call `clear_redo`.
    pub(super) fn push(&mut self, snapshot: KicadSchematic) {
        self.undo_stack.push(snapshot);
    }

    /// Returns `true` if there is at least one state to undo to.
    pub(super) fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Returns `true` if there is at least one state to redo to.
    pub(super) fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Pop the most recent undo snapshot, saving the current state for redo.
    ///
    /// Returns `None` when the undo stack is empty.
    pub(super) fn undo(&mut self, current: KicadSchematic) -> Option<KicadSchematic> {
        let previous = self.undo_stack.pop()?;
        self.redo_stack.push(current);
        Some(previous)
    }

    /// Pop the most recent redo snapshot, saving the current state for undo.
    ///
    /// Returns `None` when the redo stack is empty.
    pub(super) fn redo(&mut self, current: KicadSchematic) -> Option<KicadSchematic> {
        let next = self.redo_stack.pop()?;
        self.undo_stack.push(current);
        Some(next)
    }

    /// Discard all redo snapshots.  Called after a new edit so the forward
    /// branch is no longer valid.
    pub(super) fn clear_redo(&mut self) {
        self.redo_stack.clear();
    }

    /// Remove all history (e.g. when a new document is loaded).
    pub(super) fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_schematic() -> KicadSchematic {
        KicadSchematic {
            source: String::new(),
            version: None,
            generator: None,
            generator_version: None,
            uuid: None,
            paper: None,
            title_block: None,
            library_symbols: Vec::new(),
            bus_aliases: Vec::new(),
            symbols: Vec::new(),
            wires: Vec::new(),
            buses: Vec::new(),
            bus_entries: Vec::new(),
            net_chains: Vec::new(),
            graphics: Vec::new(),
            images: Vec::new(),
            tables: Vec::new(),
            rule_areas: Vec::new(),
            groups: Vec::new(),
            directive_labels: Vec::new(),
            labels: Vec::new(),
            sheets: Vec::new(),
            no_connects: Vec::new(),
            text_items: Vec::new(),
            text_boxes: Vec::new(),
            junctions: Vec::new(),
            sheet_instances: Vec::new(),
            symbol_instances: Vec::new(),
            embedded_fonts: None,
        }
    }

    #[test]
    fn push_and_undo_round_trip() {
        let mut history = EditHistory::default();
        assert!(!history.can_undo());
        assert!(!history.can_redo());

        history.push(empty_schematic());
        assert!(history.can_undo());

        let restored = history.undo(empty_schematic());
        assert!(restored.is_some());
        assert!(history.can_redo());
        assert!(!history.can_undo());
    }

    #[test]
    fn redo_after_undo() {
        let mut history = EditHistory::default();
        history.push(empty_schematic());
        history.push(empty_schematic());

        let prev = history.undo(empty_schematic());
        assert!(prev.is_some());

        let next = history.redo(empty_schematic());
        assert!(next.is_some());
    }

    #[test]
    fn clear_redo_discards_branch() {
        let mut history = EditHistory::default();
        history.push(empty_schematic());
        history.undo(empty_schematic());
        assert!(history.can_redo());

        history.clear_redo();
        assert!(!history.can_redo());
    }
}
