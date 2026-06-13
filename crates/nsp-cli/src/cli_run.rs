//! CLI 子命令：仿真运行相关。
//!
//! 包含 `run`（直接运行网表）和 `run-schematic`（从原理图生成网表并运行）
//! 两个子命令的实现。

use nsp_core::{OslError, OslResult, RunStatus, write_text};
use nsp_schema::read_schematic_with_libraries;
use nsp_sim::{NgspiceCliBackend, SimulatorBackend};
use std::fs;
use std::path::{Path, PathBuf};

use crate::{default_run_dir, finalize_run_output, flag_value, positional};

/// 运行 SPICE 网表文件。
///
/// 用法: `nk run <netlist.cir> [--output <dir>] [--ngspice <path>]`
pub(crate) fn run_command(args: &[String]) -> OslResult<i32> {
    let input = positional(args, 0, "missing netlist path for 'nk run'")?;
    let ngspice = flag_value(args, "--ngspice").unwrap_or_else(|| "ngspice".to_string());
    let output_dir = flag_value(args, "--output")
        .map(PathBuf::from)
        .unwrap_or_else(|| default_run_dir(input));

    // Normalize SPICE model types and resolve include paths for .cir files
    let input_path = Path::new(input);
    let input_dir = std::fs::canonicalize(input_path)
        .map(|p| p.parent().unwrap_or(Path::new(".")).to_path_buf())
        .unwrap_or_else(|_| input_path.parent().unwrap_or(Path::new(".")).to_path_buf());
    let raw_netlist = std::fs::read_to_string(input_path)
        .map_err(|err| OslError::io(format!("read {}", input_path.display()), err))?;
    let netlist = normalize_spice_models(&raw_netlist);
    let netlist_resolved = resolve_include_paths(&netlist, &input_dir);
    let netlist = normalize_included_lib_files(&netlist_resolved);

    // Write normalized netlist to output dir
    fs::create_dir_all(&output_dir)
        .map_err(|err| OslError::io(format!("create {}", output_dir.display()), err))?;
    let netlist_path = output_dir.join("input.cir");
    write_text(&netlist_path, &netlist)?;

    let backend = NgspiceCliBackend::new(ngspice);
    let mut metadata = backend.run(&netlist_path, &output_dir)?;
    let metadata_output_dir = PathBuf::from(&metadata.output_dir);
    finalize_run_output(&metadata_output_dir, &mut metadata)?;

    println!(
        "{} {} in {} ms -> {}",
        metadata.status.as_str().to_uppercase(),
        input,
        metadata.duration_ms,
        output_dir.display()
    );

    Ok(if metadata.status == RunStatus::Passed {
        0
    } else {
        2
    })
}

/// 从原理图生成 SPICE 网表并运行仿真。
///
/// 这是主要的端到端工作流：原理图 → 网表 → 仿真 → 结果。
///
/// 用法: `nk run-schematic <file.nsp_sch> [--output <dir>] [--ngspice <path>]`
pub(crate) fn run_schematic_command(args: &[String]) -> OslResult<i32> {
    let input = positional(args, 0, "missing schematic path for 'nk run-schematic'")?;
    let ngspice = flag_value(args, "--ngspice").unwrap_or_else(|| "ngspice".to_string());
    let output_dir = flag_value(args, "--output")
        .map(PathBuf::from)
        .unwrap_or_else(|| default_run_dir(input));
    let input_path = Path::new(input);
    let extension = input_path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();

    if !matches!(extension.as_str(), "nsp_sch" | "kicad_sch") {
        return Err(OslError::InvalidInput(format!(
            "{} is not a supported schematic file (.nsp_sch)",
            input_path.display()
        )));
    }

    // 加载原理图并生成 SPICE 网表
    let schematic = read_schematic_with_libraries(input_path)?;
    let netlist_raw = schematic.to_spice_netlist()?;
    let schematic_dir = std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(input_path.parent().unwrap_or(Path::new(".")));
    let netlist_normalized = normalize_spice_models(&netlist_raw);
    let netlist_resolved = resolve_include_paths(&netlist_normalized, &schematic_dir);
    let netlist = normalize_included_lib_files(&netlist_resolved);

    // 写入网表文件
    fs::create_dir_all(&output_dir)
        .map_err(|err| OslError::io(format!("create {}", output_dir.display()), err))?;
    let netlist_path = output_dir.join("schematic.cir");
    write_text(&netlist_path, &netlist)?;
    println!("netlist -> {}", netlist_path.display());

    // 运行仿真
    let backend = NgspiceCliBackend::new(ngspice);
    let mut metadata = backend.run(&netlist_path, &output_dir)?;
    let metadata_output_dir = PathBuf::from(&metadata.output_dir);
    finalize_run_output(&metadata_output_dir, &mut metadata)?;

    println!(
        "{} schematic {} in {} ms -> {}",
        metadata.status.as_str().to_uppercase(),
        input,
        metadata.duration_ms,
        output_dir.display()
    );

    Ok(if metadata.status == RunStatus::Passed {
        0
    } else {
        2
    })
}

/// Resolve relative `.include` and `.lib` paths in a SPICE netlist
/// to absolute paths relative to the given base directory.
/// Normalize model type names to ngspice-compatible types.
///
/// Some EDA tools use model type names like `LPNP` (long PNP) which ngspice
/// doesn't recognize. This function maps them to standard SPICE types.
fn normalize_spice_models(netlist: &str) -> String {
    netlist
        .lines()
        .map(|line| {
            let trimmed = line.trim();
            if trimmed.to_ascii_lowercase().starts_with(".model ") {
                let mut tokens = trimmed.split_whitespace();
                let _dot_model = tokens.next(); // .model
                if let Some(_name) = tokens.next() {
                    if let Some(model_type) = tokens.next() {
                        let normalized = match model_type.to_ascii_uppercase().as_str() {
                            "LPNP" => "PNP",
                            "LNPN" => "NPN",
                            "LPMOS" => "PMOS",
                            "LNMOS" => "NMOS",
                            _ => model_type,
                        };
                        if normalized != model_type {
                            // Rebuild the line with normalized type
                            let prefix_len = trimmed.len() - trimmed.trim_start().len();
                            let prefix = &trimmed[..prefix_len];
                            let after_type = trimmed
                                [trimmed.find(model_type).unwrap() + model_type.len()..]
                                .to_string();
                            return format!(
                                "{prefix}.model {} {}{}",
                                _name, normalized, after_type
                            );
                        }
                    }
                }
            }
            line.to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Normalize model types inside included .lib files.
///
/// After include paths are resolved to absolute paths, this function
/// reads each .lib file, normalizes model types (LPNP->PNP etc.), and
/// writes it back. This ensures ngspice can process the models.
fn normalize_included_lib_files(netlist: &str) -> String {
    netlist
        .lines()
        .map(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with(".include ") || trimmed.starts_with(".lib ") {
                let path_part = trimmed
                    .trim_start_matches(".include")
                    .trim_start_matches(".lib")
                    .trim();
                let unquoted = path_part.trim_matches('"').trim_matches('\'');
                if !unquoted.is_empty() {
                    let path = std::path::Path::new(unquoted);
                    if path.exists()
                        && path.extension().map_or(false, |e| {
                            let ext = e.to_string_lossy().to_ascii_lowercase();
                            ext == "lib" || ext == "mod" || ext == "sub" || ext == "sp"
                        })
                    {
                        if let Ok(contents) = std::fs::read_to_string(path) {
                            let normalized = contents
                                .lines()
                                .map(|l| {
                                    let t = l.trim();
                                    if t.to_ascii_lowercase().starts_with(".model ") {
                                        let parts: Vec<&str> = t.split_whitespace().collect();
                                        if parts.len() >= 3 {
                                            let model_type = parts[2].to_ascii_uppercase();
                                            let normalized_type = match model_type.as_str() {
                                                "LPNP" => "PNP",
                                                "LNPN" => "NPN",
                                                "LPMOS" => "PMOS",
                                                "LNMOS" => "NMOS",
                                                _ => parts[2],
                                            };
                                            if normalized_type != parts[2] {
                                                let prefix_len = l.len() - l.trim_start().len();
                                                let prefix = &l[..prefix_len];
                                                let after_type = l
                                                    [l.find(parts[2]).unwrap() + parts[2].len()..]
                                                    .to_string();
                                                return format!(
                                                    "{prefix}.model {} {}{}",
                                                    parts[1], normalized_type, after_type
                                                );
                                            }
                                        }
                                    }
                                    l.to_string()
                                })
                                .collect::<Vec<_>>()
                                .join("\n");
                            if normalized != contents {
                                let _ = std::fs::write(path, normalized);
                            }
                        }
                    }
                }
            }
            line.to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Expand environment variables (${VAR} and $VAR) in include paths.
///
/// Some tools use `${KICAD7_SYMBOL_DIR}` and similar variables in Sim.Library
/// properties. This function resolves them to actual environment variable values.
fn expand_env_vars(path: &str) -> String {
    let mut result = path.to_string();
    // Expand ${VAR} style variables
    while let Some(start) = result.find("${") {
        if let Some(end) = result[start + 2..].find('}') {
            let var_name = &result[start + 2..start + 2 + end];
            if let Ok(val) = std::env::var(var_name) {
                result = format!("{}{}{}", &result[..start], val, &result[start + 3 + end..]);
            } else {
                break;
            }
        } else {
            break;
        }
    }
    result
}

/// Convert Windows-style paths (D:\path) to Unix paths.
///
/// Some tools on Windows use backslash paths. When running on Linux,
/// these need to be converted or stripped to bare filenames.
fn normalize_windows_path(path: &str) -> String {
    let expanded = expand_env_vars(path);
    // If it's a Windows absolute path like D:\Spice_general\...
    // and we're on Linux, the path won't exist. Return just the filename.
    let unix_path = expanded.replace('\\', "/");
    if unix_path.len() > 2 && unix_path.as_bytes()[1] == b':' {
        // Windows absolute path — extract just the filename for local lookup
        if let Some(filename) = unix_path.rsplit('/').next() {
            return filename.to_string();
        }
    }
    unix_path
}

fn resolve_include_paths(netlist: &str, base_dir: &Path) -> String {
    netlist
        .lines()
        .map(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with(".include ")
                || trimmed.starts_with(".lib ") && !trimmed.contains("${")
            {
                let directive_end = trimmed.find(char::is_whitespace).unwrap_or(trimmed.len());
                let path_part = trimmed[directive_end..].trim();
                let unquoted = path_part.trim_matches('"').trim_matches('\'');
                if !unquoted.is_empty()
                    && !Path::new(unquoted).is_absolute()
                    && !unquoted.contains("$")
                {
                    let expanded = expand_env_vars(unquoted);
                    let normalized = normalize_windows_path(&expanded);
                    let search = if expanded != unquoted {
                        expanded.as_str()
                    } else {
                        unquoted
                    };
                    let absolute = if Path::new(&search).is_absolute() {
                        PathBuf::from(&search)
                    } else {
                        base_dir.join(&search)
                    };
                    // Also try just the filename in base dir (for Windows paths)
                    let _fallback = base_dir.join(&normalized);
                    if absolute.exists() {
                        let abs_str = absolute.to_string_lossy().to_string();
                        let prefix = &trimmed[..directive_end];
                        let rest = &trimmed[directive_end..];
                        let resolved = rest.replace(unquoted, &abs_str);
                        return format!("{prefix}{resolved}");
                    }
                }
            }
            line.to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}
