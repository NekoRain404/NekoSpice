use crate::report_summary::{GuiReportSummary, GuiReportSummaryState};
use eframe::egui;

pub(super) fn draw_simulation_report_panel(ui: &mut egui::Ui, report: &GuiReportSummaryState) {
    match report {
        GuiReportSummaryState::Ready(summary) => draw_ready_report_summary(ui, summary),
        GuiReportSummaryState::Missing(message) => {
            ui.label("Report: missing");
            ui.monospace(message);
        }
    }
}

fn draw_ready_report_summary(ui: &mut egui::Ui, summary: &GuiReportSummary) {
    ui.label("Report");
    egui::Grid::new("simulation_report_summary")
        .num_columns(2)
        .spacing(egui::Vec2::new(8.0, 2.0))
        .show(ui, |ui| {
            ui.label("HTML");
            ui.strong(&summary.report_file);
            ui.end_row();

            ui.label("Source");
            if let Some(source_file) = &summary.source_file {
                ui.monospace(source_file);
            } else {
                ui.label("-");
            }
            ui.end_row();

            ui.label("Kind");
            ui.label(summary.source_kind.unwrap_or("report"));
            ui.end_row();

            ui.label("Mode");
            ui.label(if summary.reused_existing_html {
                "existing html"
            } else {
                "generated fallback"
            });
            ui.end_row();

            ui.label("Size");
            ui.label(format_bytes(summary.size_bytes));
            ui.end_row();
        });
}

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;

    if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

#[cfg(test)]
mod tests {
    use super::format_bytes;

    #[test]
    fn formats_report_size() {
        assert_eq!(format_bytes(24), "24 B");
        assert_eq!(format_bytes(2048), "2.0 KB");
        assert_eq!(format_bytes(5 * 1024 * 1024), "5.0 MB");
    }
}
