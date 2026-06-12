use crate::app::NekoSpiceApp;
use crate::app::localization::UiText;
use super::widgets::report_status_card;
use crate::app::theme::StudioTheme;
use eframe::egui;

impl NekoSpiceApp {
    pub(crate) fn draw_reference_measurement_rows(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        if ui.available_width() >= 720.0 {
            ui.columns(4, |columns| {
                self.draw_reference_measurement_status_card(&mut columns[0], 0);
                self.draw_reference_measurement_status_card(&mut columns[1], 1);
                self.draw_reference_measurement_status_card(&mut columns[2], 2);
                self.draw_reference_measurement_status_card(&mut columns[3], 3);
            });
        } else {
            ui.columns(2, |columns| {
                self.draw_reference_measurement_status_card(&mut columns[0], 0);
                self.draw_reference_measurement_status_card(&mut columns[1], 1);
            });
            ui.add_space(6.0);
            ui.columns(2, |columns| {
                self.draw_reference_measurement_status_card(&mut columns[0], 2);
                self.draw_reference_measurement_status_card(&mut columns[1], 3);
            });
        }
        ui.add_space(8.0);
        egui::Grid::new("reports_reference_measurements_table")
            .num_columns(6)
            .spacing(egui::Vec2::new(12.0, 4.0))
            .striped(true)
            .show(ui, |ui| {
                ui.strong(self.text(UiText::Label));
                ui.strong(self.text(UiText::Kind));
                ui.strong(self.text(UiText::LastValue));
                ui.strong(self.text(UiText::Units));
                ui.strong(self.text(UiText::Tolerance));
                ui.strong(self.text(UiText::StatusConsole));
                ui.end_row();
                for row in reference_measurements() {
                    ui.label(row.name);
                    ui.label(row.kind);
                    ui.monospace(row.value);
                    ui.label(row.unit);
                    ui.label(StudioTheme::accent_for(mode, row.margin));
                    ui.label(StudioTheme::accent_for(mode, "PASS"));
                    ui.end_row();
                }
            });
    }

    fn draw_reference_measurement_status_card(&self, ui: &mut egui::Ui, index: usize) {
        let mode = self.theme_mode();
        match index {
            0 => report_status_card(
                ui,
                mode,
                self.text(UiText::PassRate),
                "100%",
                "28 passed",
                true,
            ),
            1 => report_status_card(
                ui,
                mode,
                self.text(UiText::TotalMeasurements),
                "28",
                "0 failed",
                false,
            ),
            2 => report_status_card(
                ui,
                mode,
                self.text(UiText::WorstMargin),
                "+12.4%",
                "phase margin",
                false,
            ),
            _ => report_status_card(
                ui,
                mode,
                self.text(UiText::CriticalFailures),
                "0",
                "no waivers",
                false,
            ),
        }
    }
}

struct ReferenceMeasurement {
    name: &'static str,
    kind: &'static str,
    value: &'static str,
    unit: &'static str,
    margin: &'static str,
}

fn reference_measurements() -> [ReferenceMeasurement; 8] {
    [
        ReferenceMeasurement {
            name: "DC Gain",
            kind: "DC",
            value: "82.45",
            unit: "dB",
            margin: "+3.07%",
        },
        ReferenceMeasurement {
            name: "UGF",
            kind: "AC",
            value: "2.14",
            unit: "MHz",
            margin: "+7.00%",
        },
        ReferenceMeasurement {
            name: "Phase Margin",
            kind: "AC",
            value: "68.2",
            unit: "deg",
            margin: "+13.67%",
        },
        ReferenceMeasurement {
            name: "Gain Margin",
            kind: "AC",
            value: "14.6",
            unit: "dB",
            margin: "+46.00%",
        },
        ReferenceMeasurement {
            name: "Slew Rise",
            kind: "TRAN",
            value: "1.18",
            unit: "V/us",
            margin: "+18.00%",
        },
        ReferenceMeasurement {
            name: "Settling",
            kind: "TRAN",
            value: "2.35",
            unit: "us",
            margin: "+21.67%",
        },
        ReferenceMeasurement {
            name: "THD @1kHz",
            kind: "AC",
            value: "-92.3",
            unit: "dB",
            margin: "+15.38%",
        },
        ReferenceMeasurement {
            name: "CMRR",
            kind: "AC",
            value: "96.1",
            unit: "dB",
            margin: "+20.13%",
        },
    ]
}
