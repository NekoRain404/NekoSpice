//! Quick Start templates — pre-configured analysis setups for common circuits.
//!
//! Provides one-click access to common simulation configurations:
//! - RC Filter: transient analysis with appropriate time scale
//! - Op-Amp Bandwidth: AC analysis for frequency response
//! - DC Transfer: transfer characteristic measurement
//! - Power Supply: transient startup analysis
//! - Noise Analysis: frequency-domain noise floor
//! - Distortion Analysis: harmonic distortion measurement
//! - Sensitivity Analysis: parameter sensitivity sweep

use crate::app::NekoSpiceApp;
use crate::app::theme::StudioTheme;
use eframe::egui;
use osl_kicad::KicadSimulationDirectiveKind;
use super::state::AnalysisParams;

/// A quick-start template with analysis kind and parameter factory.
struct QuickTemplate {
    name: &'static str,
    description: &'static str,
    analysis_kind: KicadSimulationDirectiveKind,
    make_params: fn() -> AnalysisParams,
}

// ── Template parameter factories ────────────────────────────────────

fn rc_lowpass_params() -> AnalysisParams {
    AnalysisParams::Tran {
        tstep: "10u".into(), tstop: "5m".into(),
        tstart: "0".into(), tmax: "0".into(), uic: false,
    }
}

fn opamp_ac_params() -> AnalysisParams {
    AnalysisParams::Ac {
        sweep_type: "dec".into(), npoints: "100".into(),
        fstart: "1".into(), fstop: "100Meg".into(),
    }
}

fn dc_transfer_params() -> AnalysisParams {
    AnalysisParams::Dc {
        source: "V1".into(), vstart: "0".into(),
        vstop: "5".into(), vincr: "0.01".into(),
    }
}

fn power_startup_params() -> AnalysisParams {
    AnalysisParams::Tran {
        tstep: "1u".into(), tstop: "10m".into(),
        tstart: "0".into(), tmax: "0".into(), uic: true,
    }
}

fn op_point_params() -> AnalysisParams {
    AnalysisParams::Op
}

fn noise_floor_params() -> AnalysisParams {
    AnalysisParams::Noise {
        output: "V(out)".into(), input_source: "V(src)".into(),
        sweep_type: "dec".into(), npoints: "50".into(),
        fstart: "1".into(), fstop: "10Meg".into(),
    }
}

fn distortion_params() -> AnalysisParams {
    AnalysisParams::Disto {
        fstart: "1".into(), fstop: "100k".into(),
        fstep: "0".into(), maxharmonic: "3".into(),
    }
}

fn sensitivity_params() -> AnalysisParams {
    AnalysisParams::Sens {
        output: "V(out)".into(),
    }
}

fn broadband_ac_params() -> AnalysisParams {
    AnalysisParams::Ac {
        sweep_type: "dec".into(), npoints: "200".into(),
        fstart: "10".into(), fstop: "1G".into(),
    }
}

fn step_response_params() -> AnalysisParams {
    AnalysisParams::Tran {
        tstep: "1n".into(), tstop: "100u".into(),
        tstart: "0".into(), tmax: "1n".into(), uic: true,
    }
}

// ── Template definitions ────────────────────────────────────────────

fn templates() -> Vec<QuickTemplate> {
    vec![
        // Time Domain
        QuickTemplate {
            name: "RC Low-Pass",
            description: "1kHz cutoff, 10kHz source",
            analysis_kind: KicadSimulationDirectiveKind::Tran,
            make_params: rc_lowpass_params,
        },
        QuickTemplate {
            name: "Step Response",
            description: "Fast transient, 100us, UIC, 1ns timestep",
            analysis_kind: KicadSimulationDirectiveKind::Tran,
            make_params: step_response_params,
        },
        // Frequency Domain
        QuickTemplate {
            name: "Op-Amp AC",
            description: "1Hz-100MHz frequency sweep",
            analysis_kind: KicadSimulationDirectiveKind::Ac,
            make_params: opamp_ac_params,
        },
        QuickTemplate {
            name: "Broadband AC",
            description: "10Hz-1GHz, 200 points/decade",
            analysis_kind: KicadSimulationDirectiveKind::Ac,
            make_params: broadband_ac_params,
        },
        // DC
        QuickTemplate {
            name: "DC Transfer",
            description: "0-5V sweep, 10mV steps",
            analysis_kind: KicadSimulationDirectiveKind::Dc,
            make_params: dc_transfer_params,
        },
        QuickTemplate {
            name: "Operating Point",
            description: "DC bias point analysis",
            analysis_kind: KicadSimulationDirectiveKind::Op,
            make_params: op_point_params,
        },
        QuickTemplate {
            name: "Power Startup",
            description: "10ms transient, UIC enabled",
            analysis_kind: KicadSimulationDirectiveKind::Tran,
            make_params: power_startup_params,
        },
        // Advanced
        QuickTemplate {
            name: "Noise Floor",
            description: "1Hz-10MHz noise spectral density",
            analysis_kind: KicadSimulationDirectiveKind::Noise,
            make_params: noise_floor_params,
        },
        QuickTemplate {
            name: "Distortion",
            description: "Harmonic distortion analysis",
            analysis_kind: KicadSimulationDirectiveKind::Disto,
            make_params: distortion_params,
        },
        QuickTemplate {
            name: "Sensitivity",
            description: "Parameter sensitivity analysis",
            analysis_kind: KicadSimulationDirectiveKind::Sens,
            make_params: sensitivity_params,
        },
    ]
}

pub(crate) fn draw_quick_start_panel(
    app: &mut NekoSpiceApp,
    ui: &mut egui::Ui,
    mode: crate::app::theme::StudioThemeMode,
) -> bool {
    let palette = StudioTheme::palette(mode);
    let mut applied = false;

    StudioTheme::panel_frame_for(mode).show(ui, |ui| {
        ui.label(StudioTheme::section_title_for(mode, "Quick Start"));
        ui.add_space(2.0);
        ui.label(StudioTheme::muted_for(
            mode,
            "Common analysis presets for quick setup",
        ));
        ui.add_space(4.0);

        for template in templates() {
            let btn = egui::Button::new(
                egui::RichText::new(template.name).color(palette.text).size(12.0),
            )
            .fill(palette.panel_soft)
            .stroke(egui::Stroke::new(1.0, palette.border))
            .min_size(egui::Vec2::new(ui.available_width(), 32.0));

            let resp = ui.add(btn);
            if resp.on_hover_text(template.description).clicked() {
                app.simulation_panel.directive_kind = template.analysis_kind;
                app.simulation_panel.analysis_params = (template.make_params)();
                applied = true;
            }
        }
    });

    applied
}
