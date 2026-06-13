//! Domain-focused tests for nsp-schema.

use crate::{NspAt, NspSchematicEdit, NspSimulationDirectiveKind, parse_schematic, read_schematic};
use std::path::Path;

#[test]
fn sets_structured_simulation_directives_and_roundtrips() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap();
    let mut schematic =
        read_schematic(&workspace_root.join("examples/schema_schematic/rc.nsp_sch")).unwrap();

    schematic
        .apply_edit(NspSchematicEdit::SetSimulationDirective {
            kind: NspSimulationDirectiveKind::Tran,
            body: "2u 2m".to_string(),
            at: Some(NspAt {
                x: 30.48,
                y: 20.32,
                rotation: 0.0,
            }),
            uuid: Some("aaaaaaaa-0000-4000-8000-000000000001".to_string()),
        })
        .unwrap();
    schematic
        .apply_edit(NspSchematicEdit::SetSimulationDirective {
            kind: NspSimulationDirectiveKind::Save,
            body: "v(out)".to_string(),
            at: Some(NspAt {
                x: 30.48,
                y: 25.4,
                rotation: 0.0,
            }),
            uuid: Some("aaaaaaaa-0000-4000-8000-000000000002".to_string()),
        })
        .unwrap();

    let directives = schematic.simulation_directives();
    assert!(directives.iter().any(|directive| {
        directive.kind == NspSimulationDirectiveKind::Tran
            && directive.text == ".tran 2u 2m"
            && directive.uuid.as_deref() == Some("77777777-7777-7777-7777-777777777777")
    }));
    assert!(directives.iter().any(|directive| {
        directive.kind == NspSimulationDirectiveKind::Save
            && directive.text == ".save v(out)"
            && directive.uuid.as_deref() == Some("aaaaaaaa-0000-4000-8000-000000000002")
    }));

    let exported = schematic.to_schematic_sexpr();
    assert!(exported.contains("(text \".tran 2u 2m\""));
    assert!(exported.contains("(text \".save v(out)\""));
    let reparsed = parse_schematic(&exported, "simulation_directives.nsp_sch").unwrap();
    assert!(reparsed.simulation_directives().iter().any(|directive| {
        directive.kind == NspSimulationDirectiveKind::Save && directive.text == ".save v(out)"
    }));
}
