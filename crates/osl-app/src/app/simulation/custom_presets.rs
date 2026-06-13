//! Custom simulation presets — save and load user-defined presets to disk.
//!
//! Presets are stored as simple text files in the application config directory.
//! Each preset stores the solver options (temperature, tolerances, method, etc.)
//! so users can quickly switch between their common configurations.

use crate::app::NekoSpiceApp;
use super::sim_options::SimOptions;
use crate::app::theme::StudioTheme;
use eframe::egui;
use std::path::PathBuf;

/// Get the presets directory path.
fn presets_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let base = PathBuf::from(home).join(".config").join("nekospice").join("presets");
    std::fs::create_dir_all(&base).ok();
    base
}

/// Serialize SimOptions to a simple key=value text format.
fn serialize_preset(opts: &SimOptions, name: &str) -> String {
    let mut lines = vec![format!("name={}", name)];
    lines.push(format!("temperature={}", opts.temperature));
    lines.push(format!("tnom={}", opts.tnom));
    lines.push(format!("method={}", opts.method));
    lines.push(format!("reltol={}", opts.reltol));
    lines.push(format!("abstol={}", opts.abstol));
    lines.push(format!("vntol={}", opts.vntol));
    lines.push(format!("gmin={}", opts.gmin));
    lines.push(format!("chgtol={}", opts.chgtol));
    lines.push(format!("pivtol={}", opts.pivtol));
    lines.push(format!("pivrel={}", opts.pivrel));
    lines.push(format!("itl1={}", opts.itl1));
    lines.push(format!("itl2={}", opts.itl2));
    lines.push(format!("itl4={}", opts.itl4));
    lines.push(format!("itl5={}", opts.itl5));
    lines.push(format!("min_timestep={}", opts.min_timestep));
    lines.push(format!("srcsteps={}", opts.srcsteps));
    lines.push(format!("gminsteps={}", opts.gminsteps));
    lines.push(format!("numdgt={}", opts.numdgt));
    lines.join("\n")
}

/// Deserialize a key=value text format back to SimOptions.
fn deserialize_preset(content: &str) -> (String, SimOptions) {
    let mut opts = SimOptions::default();
    let mut name = "custom".to_string();
    for line in content.lines() {
        if let Some((key, value)) = line.split_once('=') {
            match key {
                "name" => name = value.to_string(),
                "temperature" => opts.temperature = value.to_string(),
                "tnom" => opts.tnom = value.to_string(),
                "method" => opts.method = value.to_string(),
                "reltol" => opts.reltol = value.to_string(),
                "abstol" => opts.abstol = value.to_string(),
                "vntol" => opts.vntol = value.to_string(),
                "gmin" => opts.gmin = value.to_string(),
                "chgtol" => opts.chgtol = value.to_string(),
                "pivtol" => opts.pivtol = value.to_string(),
                "pivrel" => opts.pivrel = value.to_string(),
                "itl1" => opts.itl1 = value.to_string(),
                "itl2" => opts.itl2 = value.to_string(),
                "itl4" => opts.itl4 = value.to_string(),
                "itl5" => opts.itl5 = value.to_string(),
                "min_timestep" => opts.min_timestep = value.to_string(),
                "srcsteps" => opts.srcsteps = value.to_string(),
                "gminsteps" => opts.gminsteps = value.to_string(),
                "numdgt" => opts.numdgt = value.to_string(),
                _ => {}
            }
        }
    }
    (name, opts)
}

/// List all saved custom presets from disk.
fn list_custom_presets() -> Vec<String> {
    let dir = presets_dir();
    let mut names = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "preset") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    names.push(stem.to_string());
                }
            }
        }
    }
    names.sort();
    names
}

impl NekoSpiceApp {
    /// Draw the custom preset manager in the profile editor.
    pub(crate) fn draw_custom_presets_panel(
        &mut self,
        ui: &mut egui::Ui,
        mode: crate::app::theme::StudioThemeMode,
    ) {
        let palette = StudioTheme::palette(mode);

        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(mode, "Custom Presets"));
            ui.add_space(4.0);

            // Save current as custom preset
            ui.horizontal(|ui| {
                ui.label(StudioTheme::muted_for(mode, "Save current as:"));
                ui.text_edit_singleline(&mut self.custom_preset_name);
                if ui.button("Save").clicked()
                    && !self.custom_preset_name.trim().is_empty()
                {
                    let name = self.custom_preset_name.trim().to_string();
                    let opts = &self.simulation_profile_editor.options;
                    let content = serialize_preset(opts, &name);
                    let path = presets_dir().join(format!("{}.preset", name));
                    if std::fs::write(&path, &content).is_ok() {
                        self.status_message = Some(format!("Preset '{}' saved", name));
                        self.simulation_profile_editor.active_preset = name;
                    }
                }
            });

            ui.add_space(4.0);

            // List saved custom presets
            let custom = list_custom_presets();
            if custom.is_empty() {
                ui.label(StudioTheme::muted_for(mode, "No custom presets saved yet."));
            } else {
                for name in &custom {
                    let active = self.simulation_profile_editor.active_preset == *name;
                    ui.horizontal(|ui| {
                        let btn = if active {
                            egui::Button::new(
                                egui::RichText::new(name.as_str()).strong().color(palette.text),
                            )
                            .fill(palette.accent_soft)
                            .stroke(egui::Stroke::new(1.0, palette.accent))
                        } else {
                            egui::Button::new(
                                egui::RichText::new(name.as_str()).color(palette.text_muted),
                            )
                            .fill(palette.panel_soft)
                            .stroke(egui::Stroke::new(1.0, palette.border))
                        };
                        if ui.add(btn).clicked() {
                            // Load preset from disk
                            let path = presets_dir().join(format!("{}.preset", name));
                            if let Ok(content) = std::fs::read_to_string(&path) {
                                let (loaded_name, opts) = deserialize_preset(&content);
                                self.simulation_profile_editor.options = opts;
                                self.simulation_profile_editor.active_preset = loaded_name;
                                self.save_simulation_settings();
                            }
                        }
                        // Delete button
                        if ui.small_button("x").on_hover_text("Delete preset").clicked() {
                            let path = presets_dir().join(format!("{}.preset", name));
                            let _ = std::fs::remove_file(&path);
                            self.status_message = Some(format!("Preset '{}' deleted", name));
                        }
                    });
                }
            }
        });
    }
}
