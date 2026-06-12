//! Simulation history — tracks the last N runs for comparison and review.
//!
//! Each entry records the run metadata, analysis type, backend, and key
//! settings so users can compare runs side-by-side and re-run with
//! the same configuration.

use crate::simulation::GuiSimulationRun;
use osl_core::RunStatus;
use std::time::{SystemTime, UNIX_EPOCH};

/// Maximum number of runs retained in history.
const MAX_HISTORY: usize = 20;

/// A single historical simulation run entry.
#[derive(Debug, Clone)]
pub(crate) struct SimulationHistoryEntry {
    /// Timestamp (Unix seconds) when the run completed.
    pub(crate) completed_at: u64,
    /// Analysis type used (e.g. ".tran", ".ac", ".dc", ".op").
    pub(crate) analysis_type: String,
    /// Analysis body/parameters (e.g. "1u 1m").
    pub(crate) analysis_body: String,
    /// Backend engine used ("ngspice" or "Xyce").
    pub(crate) backend: String,
    /// Run duration in milliseconds.
    pub(crate) duration_ms: u128,
    /// Whether the run passed or failed.
    pub(crate) status: RunStatus,
    /// Output directory path for artifact access.
    pub(crate) output_dir: String,
    /// Key settings snapshot (temperature, method, RELTOL).
    pub(crate) settings_summary: String,
}

impl SimulationHistoryEntry {
    /// Create a new history entry from a completed simulation run.
    pub(crate) fn from_run(
        run: &GuiSimulationRun,
        analysis_type: &str,
        analysis_body: &str,
        backend: &str,
        settings_summary: String,
    ) -> Self {
        let completed_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Self {
            completed_at,
            analysis_type: analysis_type.to_string(),
            analysis_body: analysis_body.to_string(),
            backend: backend.to_string(),
            duration_ms: run.metadata.duration_ms as u128,
            status: run.metadata.status,
            output_dir: run.output_dir.display().to_string(),
            settings_summary,
        }
    }

    /// Formatted timestamp for display.
    pub(crate) fn time_label(&self) -> String {
        // Simple HH:MM:SS format
        let secs = self.completed_at;
        let h = (secs / 3600) % 24;
        let m = (secs / 60) % 60;
        let s = secs % 60;
        format!("{:02}:{:02}:{:02}", h, m, s)
    }

    /// Analysis directive string for display.
    pub(crate) fn analysis_label(&self) -> String {
        if self.analysis_body.trim().is_empty() {
            self.analysis_type.clone()
        } else {
            format!("{} {}", self.analysis_type, self.analysis_body.trim())
        }
    }

    /// Status dot color helper.
    pub(crate) fn status_color(
        &self,
        palette: &crate::app::theme::StudioPalette,
    ) -> eframe::egui::Color32 {
        match self.status {
            RunStatus::Passed => palette.success,
            RunStatus::Failed => palette.danger,
        }
    }

    /// Status label.
    pub(crate) fn status_label(&self) -> &'static str {
        match self.status {
            RunStatus::Passed => "Passed",
            RunStatus::Failed => "Failed",
        }
    }
}

/// Simulation run history manager.
#[derive(Debug, Default)]
pub(crate) struct SimulationHistory {
    /// Historical entries, most recent first.
    entries: Vec<SimulationHistoryEntry>,
}

impl SimulationHistory {
    /// Record a new run in the history.
    pub(crate) fn record_run(
        &mut self,
        run: &GuiSimulationRun,
        analysis_type: &str,
        analysis_body: &str,
        backend: &str,
        settings_summary: String,
    ) {
        let entry = SimulationHistoryEntry::from_run(
            run,
            analysis_type,
            analysis_body,
            backend,
            settings_summary,
        );
        self.entries.insert(0, entry);
        self.entries.truncate(MAX_HISTORY);
    }

    /// Get the list of historical entries (most recent first).
    pub(crate) fn entries(&self) -> &[SimulationHistoryEntry] {
        &self.entries
    }

    /// Number of recorded runs.
    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether history is empty.
    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Clear all history.
    #[allow(dead_code)]
    pub(crate) fn clear(&mut self) {
        self.entries.clear();
    }
}
