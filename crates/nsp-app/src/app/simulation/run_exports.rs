//! Simulation file export dialogs — netlist, CSV waveform, and log export.
//!
//! Each export method opens a native file dialog and writes the
//! requested format to the user-chosen path. Extracted from
//! [`run_controller`] to keep individual files under 300 lines.

use crate::app::NekoSpiceApp;

impl NekoSpiceApp {
    /// Export SPICE netlist to .cir file via file dialog.
    pub(crate) fn export_netlist_dialog(&mut self) {
        let Some(document) = &self.document else {
            self.status_message = Some("No editable schematic loaded".to_string());
            return;
        };
        let profile = self.build_simulation_profile();
        let netlist = match document
            .spice_netlist_preview()
            .map(|raw| nsp_sim::inject_profile_directives(&raw, &profile))
        {
            Ok(n) => n,
            Err(error) => {
                self.status_message = Some(error);
                return;
            }
        };
        let dialog = rfd::FileDialog::new()
            .add_filter("SPICE Netlist", &["cir", "sp", "net"])
            .set_file_name("schematic.cir");
        if let Some(path) = dialog.save_file() {
            match std::fs::write(&path, &netlist) {
                Ok(()) => {
                    self.status_message = Some(format!("Netlist exported to {}", path.display()))
                }
                Err(error) => self.status_message = Some(format!("Export failed: {error}")),
            }
        }
    }
    /// Export waveform data to CSV file via file dialog.
    ///
    /// Reads the raw waveform from the last run's output directory and
    /// writes it as a comma-separated values file for external analysis.
    pub(crate) fn export_csv_dialog(&mut self) {
        let Some(run) = &self.simulation_panel.last_run else {
            self.status_message = Some("No simulation run available for CSV export".to_string());
            return;
        };
        let raw_path = run.output_dir.join("waveform.raw");
        if !raw_path.is_file() {
            self.status_message = Some(format!("Waveform file not found: {}", raw_path.display()));
            return;
        }
        match nsp_waveform::read_ngspice_raw(&raw_path) {
            Ok(waveform) => match waveform.to_csv() {
                Ok(csv) => {
                    let dialog = rfd::FileDialog::new()
                        .add_filter("CSV Data", &["csv"])
                        .set_file_name("waveform.csv");
                    if let Some(path) = dialog.save_file() {
                        match std::fs::write(&path, &csv) {
                            Ok(()) => {
                                self.status_message = Some(format!(
                                    "CSV exported to {} ({} bytes)",
                                    path.display(),
                                    csv.len()
                                ))
                            }
                            Err(error) => {
                                self.status_message = Some(format!("CSV export failed: {error}"))
                            }
                        }
                    }
                }
                Err(error) => {
                    self.status_message = Some(format!("CSV generation failed: {error}"));
                }
            },
            Err(error) => {
                self.status_message = Some(format!("Waveform parse failed: {error}"));
            }
        }
    }
    /// Export simulation log to a file via file dialog.
    ///
    /// Copies the ngspice or Xyce log from the last run to a user-selected path.
    pub(crate) fn export_log_dialog(&mut self) {
        let Some(run) = &self.simulation_panel.last_run else {
            self.status_message = Some("No simulation run available for log export".to_string());
            return;
        };
        let ngspice_log = run.output_dir.join("ngspice.log");
        let xyce_log = run.output_dir.join("xyce.log");
        let source = if ngspice_log.is_file() {
            ngspice_log
        } else if xyce_log.is_file() {
            xyce_log
        } else {
            self.status_message = Some("No simulation log found".to_string());
            return;
        };
        let dialog = rfd::FileDialog::new()
            .add_filter("Simulation Log", &["log", "txt"])
            .set_file_name(
                source
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("simulation.log")
                    .to_string(),
            );
        if let Some(path) = dialog.save_file() {
            match std::fs::copy(&source, &path) {
                Ok(bytes) => {
                    self.status_message = Some(format!(
                        "Log exported to {} ({} bytes)",
                        path.display(),
                        bytes
                    ))
                }
                Err(error) => self.status_message = Some(format!("Log export failed: {error}")),
            }
        }
    }
}
