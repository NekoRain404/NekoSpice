//! Simulation history — tracks the last N runs for comparison and review.
//!
//! Each entry records the run metadata, analysis type, backend, and key
//! settings so users can compare runs side-by-side and re-run with
//! the same configuration.

use crate::simulation::GuiSimulationRun;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Maximum number of runs retained in history.
const MAX_HISTORY: usize = 20;

/// A single historical simulation run entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// Whether the run passed or failed (stored as string for serialization).
    pub(crate) status_str: String,
    /// Output directory path for artifact access.
    #[allow(dead_code)]
    pub(crate) output_dir: String,
    /// Key settings snapshot (temperature, method, RELTOL).
    #[allow(dead_code)]
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
            status_str: run.metadata.status.as_str().to_string(),
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
        match self.status_str.as_str() {
            "Passed" => palette.success,
            _ => palette.danger,
        }
    }

    /// Status label.
    pub(crate) fn status_label(&self) -> &str {
        &self.status_str
    }
}

/// Simulation run history manager.
#[derive(Debug)]
pub(crate) struct SimulationHistory {
    /// Historical entries, most recent first.
    entries: Vec<SimulationHistoryEntry>,
}

impl Default for SimulationHistory {
    fn default() -> Self {
        Self::load_from_disk()
    }
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
        self.save_to_disk();
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

    /// Save history to disk.
    pub(crate) fn save_to_disk(&self) {
        let path = history_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(&self.entries) {
            let _ = std::fs::write(&path, json);
        }
    }

    /// Load history from disk.
    pub(crate) fn load_from_disk() -> Self {
        let path = history_path();
        let data = match std::fs::read_to_string(&path) {
            Ok(d) => d,
            Err(_) => return Self::default(),
        };
        let entries: Vec<SimulationHistoryEntry> = match serde_json::from_str(&data) {
            Ok(e) => e,
            Err(_) => return Self::default(),
        };
        Self { entries }
    }
}


/// Path to the history JSON file.
fn history_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join(".config")
        .join("nekospice")
        .join("simulation_history.json")
}