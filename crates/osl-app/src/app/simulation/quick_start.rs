//! Quick Start templates — pre-configured analysis setups for common circuits.
//!
//! Provides one-click access to common simulation configurations:
//! - RC Filter: transient analysis with appropriate time scale
//! - Op-Amp Bandwidth: AC analysis for frequency response
//! - DC Sweep: transfer characteristic measurement
//! - Power Supply: transient startup analysis

use crate::app::NekoSpiceApp;
use crate::app::theme::StudioTheme;
use eframe::egui;
use osl_kicad::KicadSimulationDirectiveKind;
use super::state::AnalysisParams;

struct QuickTemplate {
    name: &'static str,
    description: &'static str,
    analysis_kind: KicadSimulationDirectiveKind,
    make_params: fn() -> AnalysisParams,
}

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

fn templates() -> Vec<QuickTemplate> {
    vec![
        QuickTemplate { name: "RC Low-Pass", description: "1kHz cutoff, 10kHz source",
            analysis_kind: KicadSimulationDirectiveKind::Tran, make_params: rc_lowpass_params },
        QuickTemplate { name: "Op-Amp AC", description: "1Hz–100MHz frequency sweep",
            analysis_kind: KicadSimulationDirectiveKind::Ac, make_params: opamp_ac_params },
        QuickTemplate { name: "DC Transfer", description: "0–5V sweep, 10mV steps",
            analysis_kind: KicadSimulationDirectiveKind::Dc, make_params: dc_transfer_params },
        QuickTemplate { name: "Power Startup", description: "10ms transient, UIC enabled",
            analysis_kind: KicadSimulationDirectiveKind::Tran, make_params: power_startup_params },
        QuickTemplate { name: "Operating Point", description: "DC bias point analysis",
            analysis_kind: KicadSimulationDirectiveKind::Op, make_params: op_point_params },
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
                egui::RichText::new(template.name).color(palette.text),
            )
            .fill(palette.panel_soft)
            .stroke(egui::Stroke::new(1.0, palette.border))
            .min_size(egui::Vec2::new(ui.available_width(), 36.0));

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
