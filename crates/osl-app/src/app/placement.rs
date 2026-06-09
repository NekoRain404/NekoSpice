use super::NekoSpiceApp;
use osl_kicad::{KicadAt, KicadCanvasHit, KicadCanvasScene, KicadPoint};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SymbolPlacementState {
    pub(super) symbol_id: String,
    pub(super) keep_active: bool,
}

impl NekoSpiceApp {
    pub(super) fn start_symbol_placement(&mut self) {
        let Some(symbol_id) = self.selected_symbol_id.clone() else {
            self.status_message = Some("Select a symbol before placing".to_string());
            return;
        };
        self.placement = Some(SymbolPlacementState {
            symbol_id: symbol_id.clone(),
            keep_active: false,
        });
        self.status_message = Some(format!("Click canvas to place {symbol_id}"));
    }

    pub(super) fn cancel_symbol_placement(&mut self) {
        if self.placement.take().is_some() {
            self.status_message = Some("Canceled symbol placement".to_string());
        }
    }

    pub(super) fn place_selected_symbol_at_point(&mut self, point: KicadPoint) {
        let Some(symbol_id) = self
            .placement
            .as_ref()
            .map(|placement| placement.symbol_id.clone())
        else {
            return;
        };
        let Some(library) = &self.library else {
            self.status_message = Some("No symbol library loaded".to_string());
            return;
        };
        let Some(document) = &mut self.document else {
            self.status_message = Some("No editable schematic loaded".to_string());
            return;
        };

        let at = KicadAt {
            x: point.x,
            y: point.y,
            rotation: 0.0,
        };

        match library
            .symbol_definition(&symbol_id)
            .and_then(|symbol| document.place_symbol_from_definition(symbol.definition, at))
        {
            Ok(placement) => {
                let keep_active = self
                    .placement
                    .as_ref()
                    .is_some_and(|placement| placement.keep_active);
                let scene = document.scene();
                self.selected_hit = hit_for_symbol_reference(&scene, &placement.reference);
                self.scene = Some(scene);
                if !keep_active {
                    self.placement = None;
                }
                self.load_error = None;
                self.status_message = Some(format!(
                    "Placed {} {}",
                    placement.summary.operation, placement.summary.target
                ));
            }
            Err(error) => {
                self.status_message = Some(error);
            }
        }
    }
}

fn hit_for_symbol_reference(scene: &KicadCanvasScene, reference: &str) -> Option<KicadCanvasHit> {
    let symbol = scene
        .symbols
        .iter()
        .find(|symbol| symbol.reference == reference)?;
    if let Some(uuid) = &symbol.uuid
        && let Some(hit) = scene.item_hit_by_uuid(uuid)
    {
        return Some(hit);
    }

    Some(KicadCanvasHit {
        kind: "symbol".to_string(),
        uuid: symbol.uuid.clone(),
        label: symbol.reference.clone(),
        bounds: symbol.bounds?,
    })
}

#[cfg(test)]
mod tests {
    use crate::app::NekoSpiceApp;

    #[test]
    fn placement_mode_starts_and_cancels_from_selected_symbol() {
        let mut app = NekoSpiceApp::default();
        app.selected_symbol_id = Some("NekoSpice:R".to_string());

        app.start_symbol_placement();

        let placement = app.placement.as_ref().unwrap();
        assert_eq!(placement.symbol_id, "NekoSpice:R");
        assert!(!placement.keep_active);

        app.cancel_symbol_placement();

        assert!(app.placement.is_none());
        assert_eq!(
            app.status_message.as_deref(),
            Some("Canceled symbol placement")
        );
    }
}
