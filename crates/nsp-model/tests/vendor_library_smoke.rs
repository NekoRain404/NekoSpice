//! Smoke test: load real vendor SPICE model libraries from KiCad-Spice-Library-master.

use nsp_model::{VendorKind, import_spice_model_file, is_spice_model_file};
use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .find(|p| p.join("Cargo.toml").is_file() && p.join("crates").is_dir())
        .expect("workspace root not found")
        .to_path_buf()
}

/// Recursively count parseable SPICE model files under a directory.
fn count_parseable_models(dir: &Path) -> (usize, usize, usize) {
    let mut files = 0;
    let mut subckts = 0;
    let mut models = 0;
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let (f, s, m) = count_parseable_models(&path);
                files += f; subckts += s; models += m;
            } else if is_spice_model_file(&path) {
                if let Ok(result) = import_spice_model_file(&path) {
                    subckts += result.subckts.len();
                    models += result.models.len();
                    files += 1;
                }
            }
        }
    }
    (files, subckts, models)
}

#[test]
fn loads_diode_models() {
    let lib = workspace_root().join("KiCad-Spice-Library-master/Models/Diode/diode.lib");
    if !lib.exists() { return; }
    let result = import_spice_model_file(&lib).expect("failed to import diode.lib");
    eprintln!("diode.lib: {} subckts, {} models", result.subckts.len(), result.models.len());
    assert!(result.subckts.len() + result.models.len() > 0);
}

#[test]
fn loads_zener_models() {
    let lib = workspace_root().join("KiCad-Spice-Library-master/Models/Diode/zener.lib");
    if !lib.exists() { return; }
    let result = import_spice_model_file(&lib).expect("failed to import zener.lib");
    eprintln!("zener.lib: {} subckts, {} models", result.subckts.len(), result.models.len());
    assert!(result.subckts.len() + result.models.len() > 0);
}

#[test]
fn loads_transistor_models() {
    let dir = workspace_root().join("KiCad-Spice-Library-master/Models/Transistor");
    if !dir.exists() { return; }
    let (files, subckts, models) = count_parseable_models(&dir);
    eprintln!("Transistor/: {files} files, {subckts} subckts, {models} models");
    assert!(files > 0, "should load at least one transistor model file");
}

#[test]
fn loads_manufacturer_models() {
    let dir = workspace_root().join("KiCad-Spice-Library-master/Models/Manufacturer");
    if !dir.exists() { return; }
    let (files, subckts, _models) = count_parseable_models(&dir);
    eprintln!("Manufacturer/: {files} files, {subckts} subckts");
    assert!(files > 0, "should load at least one manufacturer model file");
}

#[test]
fn loads_digital_logic_models() {
    let dir = workspace_root().join("KiCad-Spice-Library-master/Models/Digital Logic");
    if !dir.exists() { return; }
    let (files, subckts, models) = count_parseable_models(&dir);
    eprintln!("Digital Logic/: {files} files, {subckts} subckts, {models} models");
    assert!(files > 0, "should load at least one digital logic model file");
}

#[test]
fn loads_opamp_models() {
    let dir = workspace_root().join("KiCad-Spice-Library-master/Models/Operational Amplifier");
    if !dir.exists() { return; }
    let (files, subckts, models) = count_parseable_models(&dir);
    eprintln!("OpAmp/: {files} files, {subckts} subckts, {models} models");
    assert!(files > 0, "should load at least one opamp model file");
}

#[test]
fn vendor_detection_linear_tech_is_adi() {
    let lt = workspace_root().join("KiCad-Spice-Library-master/Models/Manufacturer/Linear Technology Corporation/LinearTech.lib");
    if lt.exists() {
        assert_eq!(VendorKind::detect(&lt), VendorKind::Adi);
    }
}
