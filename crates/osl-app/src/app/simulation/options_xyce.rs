//! Xyce-specific solver options — convergence and output settings
//! unique to the Xyce parallel SPICE simulator.
//!
//! Xyce has different default parameters than ngspice:
//! - Different convergence algorithm options
//! - Xyce-specific output format controls
//! - Parallel simulation settings
//!
//! These options are only shown when the Xyce backend is selected.

use crate::app::NekoSpiceApp;
use super::profile_editor_widgets::{labeled_field, section_header};
use crate::app::theme::{StudioTheme, StudioThemeMode};
use eframe::egui;

/// Xyce-specific simulation options that extend the base SimOptions.
#[derive(Debug, Clone)]
pub(crate) struct XyceOptions {
    /// Xyce convergence method: "Newton" or "SourceStepping".
    pub convergence_method: String,
    /// Maximum Newton iterations per timestep.
    pub max_newton_iterations: String,
    /// Xyce output format: "rawfile", "rawfile4", "std".
    pub output_format: String,
    /// Whether to enable Xyce's built-in parallel partitioning.
    pub parallel_partition: bool,
    /// Number of parallel partitions (0 = auto).
    pub partition_count: String,
    /// Whether to output ASCII raw file instead of binary.
    pub ascii_rawfile: bool,
    /// Whether to output parameter sweep results to a single file.
    pub sweep_single_file: bool,
}

impl Default for XyceOptions {
    fn default() -> Self {
        Self {
            convergence_method: "Newton".to_string(),
            max_newton_iterations: "50".to_string(),
            output_format: "rawfile4".to_string(),
            parallel_partition: false,
            partition_count: "0".to_string(),
            ascii_rawfile: false,
            sweep_single_file: true,
        }
    }
}

/// Draw Xyce-specific solver options section.
/// Only visible when the Xyce backend is selected.
/// Returns `true` when any field changes.
pub(crate) fn draw_xyce_options_section(
    app: &mut NekoSpiceApp,
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
) -> bool {
    let mut changed = false;

    StudioTheme::panel_frame_for(mode).show(ui, |ui| {
        section_header(ui, mode, "Xyce Solver Options");
        ui.add_space(4.0);

        // Convergence method selector
        let palette = StudioTheme::palette(mode);
        ui.label(StudioTheme::muted_for(mode, "Convergence Method"));
        ui.horizontal(|ui| {
            for method in ["Newton", "SourceStepping"] {
                let active = app.xyce_options.convergence_method == method;
                let btn = if active {
                    egui::Button::new(egui::RichText::new(method).strong().color(palette.text))
                        .fill(palette.accent_soft)
                        .stroke(egui::Stroke::new(1.0, palette.accent))
                } else {
                    egui::Button::new(egui::RichText::new(method).color(palette.text_muted))
                        .fill(palette.panel_soft)
                        .stroke(egui::Stroke::new(1.0, palette.border))
                };
                if ui.add(btn).clicked() {
                    if app.xyce_options.convergence_method != method {
                        app.xyce_options.convergence_method = method.to_string();
                        changed = true;
                    }
                }
            }
        });

        ui.add_space(6.0);

        // Solver parameters grid
        egui::Grid::new("xyce_solver_grid")
            .num_columns(2)
            .spacing([8.0, 6.0])
            .show(ui, |ui| {
                changed |= labeled_field(
                    ui, mode,
                    "Max Newton Iters",
                    &mut app.xyce_options.max_newton_iterations,
                    100.0,
                );
                changed |= labeled_field(
                    ui, mode,
                    "Partition Count",
                    &mut app.xyce_options.partition_count,
                    100.0,
                );
            });

        ui.add_space(6.0);

        // Output format selector
        ui.label(StudioTheme::muted_for(mode, "Output Format"));
        let current_fmt = app.xyce_options.output_format.clone();
        ui.horizontal(|ui| {
            for fmt in ["rawfile4", "rawfile", "std"] {
                let active = current_fmt == fmt;
                let btn = if active {
                    egui::Button::new(egui::RichText::new(fmt).strong().color(palette.text))
                        .fill(palette.accent_soft)
                        .stroke(egui::Stroke::new(1.0, palette.accent))
                } else {
                    egui::Button::new(egui::RichText::new(fmt).color(palette.text_muted))
                        .fill(palette.panel_soft)
                        .stroke(egui::Stroke::new(1.0, palette.border))
                };
                if ui.add(btn).clicked() && app.xyce_options.output_format != fmt {
                    app.xyce_options.output_format = fmt.to_string();
                    changed = true;
                }
            }
        });

        ui.add_space(6.0);

        // Boolean toggles
        let mut parallel = app.xyce_options.parallel_partition;
        if ui.checkbox(&mut parallel, "Enable parallel partitioning").changed() {
            app.xyce_options.parallel_partition = parallel;
            changed = true;
        }
        let mut ascii = app.xyce_options.ascii_rawfile;
        if ui.checkbox(&mut ascii, "ASCII raw file output").changed() {
            app.xyce_options.ascii_rawfile = ascii;
            changed = true;
        }
        let mut single = app.xyce_options.sweep_single_file;
        if ui.checkbox(&mut single, "Sweep results to single file").changed() {
            app.xyce_options.sweep_single_file = single;
            changed = true;
        }
    });

    changed
}
