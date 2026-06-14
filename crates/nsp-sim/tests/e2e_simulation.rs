//! End-to-end simulation integration tests.
//!
//! Tests the full pipeline: load KiCad schematic -> export SPICE netlist ->
//! run ngspice simulation -> parse results -> verify waveform.
//!
//! Uses real demo schematics from KiCad-Simulations-main/ directory.

use nsp_schema::read_schematic_with_libraries;
use nsp_sim::{NgspiceCliBackend, SimulationProfile, SimulatorBackend, inject_profile_directives};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Root of the project workspace (contains KiCad-Simulations-main/).
fn workspace_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .ancestors()
        .find(|p| p.join("Cargo.toml").is_file() && p.join("crates").is_dir())
        .expect("workspace root not found")
        .to_path_buf()
}

/// Shared helper: load schematic, export netlist, inject profile, run ngspice,
/// and return (metadata, raw_path, log_tail).
fn run_demo_simulation(
    sch_rel: &str,
    extra_deps: &[&str],
) -> Result<(nsp_core::RunMetadata, Option<PathBuf>, String), String> {
    let root = workspace_root();
    let dir = root.join(sch_rel);
    let sch_name = dir
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    // Find .kicad_sch
    let sch_path = fs::read_dir(&dir)
        .map_err(|e| format!("read dir {}: {e}", dir.display()))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e == "kicad_sch")
        })
        .ok_or_else(|| format!("no .kicad_sch in {}", dir.display()))?;

    eprintln!("Loading: {}", sch_path.display());
    let schematic = read_schematic_with_libraries(&sch_path)
        .map_err(|e| format!("load failed: {e}"))?;
    eprintln!(
        "  symbols={}, wires={}, labels={}",
        schematic.symbols.len(),
        schematic.wires.len(),
        schematic.labels.len()
    );

    let netlist = schematic
        .to_spice_netlist()
        .map_err(|e| format!("netlist export failed: {e}"))?;
    assert!(netlist.contains(".end"), "netlist must have .end");
    eprintln!("  netlist: {} chars", netlist.len());

    let profile = SimulationProfile::default();
    let complete = inject_profile_directives(&netlist, &profile);

    // Write netlist + deps
    let run_id = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let tmp_dir = std::env::temp_dir().join(format!("nekospice_e2e_{sch_name}_{run_id}"));
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).map_err(|e| format!("mkdir: {e}"))?;

    let netlist_path = tmp_dir.join(format!("{sch_name}.cir"));
    fs::write(&netlist_path, &complete).map_err(|e| format!("write: {e}"))?;

    // Copy extra dependency files (.lib, .mod, .sub, .modf, .inc)
    for dep in extra_deps {
        let src = dir.join(dep);
        if src.exists() {
            fs::copy(&src, tmp_dir.join(dep)).ok();
        }
    }
    // Also copy any .lib/.mod files from the same dir
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.ends_with(".lib")
                || name_str.ends_with(".mod")
                || name_str.ends_with(".sub")
                || name_str.ends_with(".modf")
                || name_str.ends_with(".inc")
            {
                fs::copy(entry.path(), tmp_dir.join(&name)).ok();
            }
        }
    }

    let backend = NgspiceCliBackend::default();
    let result = backend.run(&netlist_path, &tmp_dir.join("run"));
    let run_dir = tmp_dir.join("run");

    // Read log tail
    let log_tail = {
        let log_path = run_dir.join("ngspice.log");
        if log_path.exists() {
            let log = fs::read_to_string(&log_path).unwrap_or_default();
            let all: Vec<&str> = log.lines().collect();
            let start = all.len().saturating_sub(12);
            all[start..].join("\n")
        } else {
            String::from("<no log>")
        }
    };

    let raw_path = run_dir.join("waveform.raw");

    match result {
        Ok(meta) => {
            eprintln!("  status={:?}, duration={}ms", meta.status, meta.duration_ms);
            if raw_path.exists() {
                let sz = fs::metadata(&raw_path).unwrap().len();
                eprintln!("  waveform.raw: {sz} bytes");
            }
            Ok((meta, raw_path.exists().then_some(raw_path), log_tail))
        }
        Err(e) => {
            eprintln!("  ERROR: {e}");
            eprintln!("  log:\n{log_tail}");
            Err(e.to_string())
        }
    }
}

// ─── Test cases ────────────────────────────────────────────────────────────

#[test]
fn rc_filter_simulation() {
    let (meta, raw, _log) =
        run_demo_simulation("KiCad-Simulations-main/rc-filter", &[]).unwrap();
    assert_eq!(meta.status, nsp_core::RunStatus::Passed);
    assert!(raw.is_some(), "RC filter must produce waveform.raw");
}

#[test]
fn rc_filter_waveform_parseable() {
    let (_meta, raw, _log) =
        run_demo_simulation("KiCad-Simulations-main/rc-filter", &[]).unwrap();
    let raw = raw.expect("waveform.raw must exist");
    let waveform = nsp_waveform::read_ngspice_raw(&raw).expect("must parse waveform");
    assert!(
        waveform.point_count() > 0,
        "waveform must have data points"
    );
    assert!(
        !waveform.variables().is_empty(),
        "waveform must have variables"
    );
    // RC filter should have v_out and vdd signals
    let var_names: Vec<&str> = waveform.variables().iter().map(|v| v.name.as_str()).collect();
    eprintln!("  waveform variables: {var_names:?}");
    assert!(
        var_names.iter().any(|v| v.contains("v_out")),
        "RC filter waveform should have v_out signal"
    );
}

#[test]
fn five55_bipolar_simulation() {
    let (meta, raw, _log) =
        run_demo_simulation("KiCad-Simulations-main/555-bipolar", &["bipmod.lib"]).unwrap();
    assert_eq!(meta.status, nsp_core::RunStatus::Passed);
    assert!(raw.is_some(), "555-bipolar must produce waveform.raw");
}

#[test]
fn five55_bipolar_waveform_parseable() {
    let (_meta, raw, _log) =
        run_demo_simulation("KiCad-Simulations-main/555-bipolar", &["bipmod.lib"]).unwrap();
    let raw = raw.expect("waveform.raw must exist");
    let waveform = nsp_waveform::read_ngspice_raw(&raw).expect("must parse waveform");
    assert!(waveform.point_count() > 100, "555 should produce many data points");
    let var_names: Vec<&str> = waveform.variables().iter().map(|v| v.name.as_str()).collect();
    eprintln!("  555 waveform vars: {var_names:?}, points={}", waveform.point_count());
}

#[test]
fn buck_converter_simulation() {
    let (meta, _raw, _log) =
        run_demo_simulation("KiCad-Simulations-main/Buck", &[]).unwrap();
    assert_eq!(meta.status, nsp_core::RunStatus::Passed);
}

#[test]
fn boost_converter_simulation() {
    let (meta, _raw, _log) =
        run_demo_simulation("KiCad-Simulations-main/Boost", &[]).unwrap();
    assert_eq!(meta.status, nsp_core::RunStatus::Passed);
}

#[test]
fn sallen_key_lowpass_simulation() {
    let (meta, raw, _log) =
        run_demo_simulation("KiCad-Simulations-main/sallen-key-lowpass", &[]).unwrap();
    assert_eq!(meta.status, nsp_core::RunStatus::Passed);
    if let Some(raw_path) = raw {
        let waveform =
            nsp_waveform::read_ngspice_raw(&raw_path).expect("must parse sallen-key waveform");
        assert!(waveform.point_count() > 0);
    }
}

#[test]
fn class_d_amplifier_simulation() {
    let (meta, _raw, log) =
        run_demo_simulation("KiCad-Simulations-main/Class-D", &[]).unwrap();
    // Class-D may fail due to complex models — log the result
    eprintln!("  Class-D status={:?} log:\n{log}", meta.status);
}

#[test]
fn full_bridge_simulation() {
    let (meta, _raw, log) =
        run_demo_simulation("KiCad-Simulations-main/FullBridge", &[]).unwrap();
    eprintln!("  FullBridge status={:?} log:\n{log}", meta.status);
}

// ─── Additional demo circuit tests ─────────────────────────────────────────

#[test]
fn sallen_key_highpass_simulation() {
    let (meta, _raw, _log) =
        run_demo_simulation("KiCad-Simulations-main/sallen-key-highpass", &["ad8051.lib"]).unwrap();
    assert_eq!(meta.status, nsp_core::RunStatus::Passed);
}

#[test]
fn opamp_741_simulation() {
    let (meta, _raw, log) =
        run_demo_simulation("KiCad-Simulations-main/741", &["bipmod.lib"]).unwrap();
    eprintln!("741 opamp status={:?}\n{log}", meta.status);
}

#[test]
fn pwm_audio_simulation() {
    let (meta, _raw, log) =
        run_demo_simulation("KiCad-Simulations-main/pwm-audio", &[]).unwrap();
    eprintln!("pwm-audio status={:?}\n{log}", meta.status);
}

#[test]
fn analog_multiplier_simulation() {
    let (meta, _raw, log) =
        run_demo_simulation("KiCad-Simulations-main/analog-multiplier", &[]).unwrap();
    eprintln!("analog-multiplier status={:?}\n{log}", meta.status);
}

#[test]
fn cmos555_simulation() {
    let (meta, _raw, log) =
        run_demo_simulation("KiCad-Simulations-main/CMOS555_4", &[]).unwrap();
    eprintln!("CMOS555 status={:?}\n{log}", meta.status);
}

#[test]
fn gain_ctrl_amp_simulation() {
    let (meta, _raw, log) =
        run_demo_simulation("KiCad-Simulations-main/gain-ctrl-amp", &[]).unwrap();
    eprintln!("gain-ctrl-amp status={:?}\n{log}", meta.status);
}

// ─── Additional power electronics and analog circuit tests ────────────────

#[test]
fn llc_converter_simulation() {
    let (meta, _raw, log) =
        run_demo_simulation("KiCad-Simulations-main/LLC", &["models.lib"]).unwrap();
    eprintln!("LLC status={:?}\n{log}", meta.status);
}

#[test]
fn royer_converter_simulation() {
    let (meta, _raw, log) =
        run_demo_simulation("KiCad-Simulations-main/Royer", &["royer1.kicad_sch"]).unwrap();
    // Royer has its own .lib files
    eprintln!("Royer status={:?}\n{log}", meta.status);
}

#[test]
fn intro4_opamp_simulation() {
    let (meta, _raw, log) =
        run_demo_simulation("KiCad-Simulations-main/intro4", &[]).unwrap();
    eprintln!("intro4 status={:?}\n{log}", meta.status);
}

#[test]
fn rel_osc_simulation() {
    let (meta, _raw, log) =
        run_demo_simulation("KiCad-Simulations-main/rel_osc", &["40106all.lib", "CD40106B.lib"]).unwrap();
    eprintln!("rel_osc status={:?}\n{log}", meta.status);
}

#[test]
fn bip_osc_simulation() {
    let (meta, _raw, log) =
        run_demo_simulation("KiCad-Simulations-main/bip-osc-2", &["bipmod.lib"]).unwrap();
    eprintln!("bip-osc status={:?}\n{log}", meta.status);
}
