//! Editable simulation options — SPICE `.options` parameters.
//!
//! Maps directly to the `SimulationProfile` fields so that the GUI
//! can edit them independently before committing to a profile build.
//!
//! Also provides named presets that apply groups of settings at once.


/// Editable simulation options shown in the profile editor.
///
/// These map directly to SPICE `.options` directives. Organized into
/// logical groups matching the SimulationProfile struct.
#[derive(Debug)]
pub(crate) struct SimOptions {
    // ── Environment ────────────────────────────────────────────────────
    /// Simulation temperature in degrees Celsius.
    pub(crate) temperature: String,
    /// Nominal temperature for model evaluation (°C).
    pub(crate) tnom: String,

    // ── Transient Solver ───────────────────────────────────────────────
    /// SPICE integration method: "Gear" or "Trap".
    pub(crate) method: String,
    /// DC operating point iteration limit (ITL1).
    pub(crate) itl1: String,
    /// DC transfer curve iteration limit (ITL2).
    pub(crate) itl2: String,
    /// Transient timestep iteration limit (ITL4).
    pub(crate) itl4: String,
    /// Transient total iteration limit (ITL5, 0 = no limit).
    pub(crate) itl5: String,
    /// Minimum allowed timestep (TRTOL). 0 = auto.
    pub(crate) min_timestep: String,
    /// Source stepping iterations (0 = auto).
    pub(crate) srcsteps: String,
    /// GMIN stepping iterations (0 = auto).
    pub(crate) gminsteps: String,

    // ── Convergence ────────────────────────────────────────────────────
    /// Relative convergence tolerance (RELTOL).
    pub(crate) reltol: String,
    /// Absolute current convergence tolerance (ABSTOL).
    pub(crate) abstol: String,
    /// Absolute voltage convergence tolerance (VNTOL).
    pub(crate) vntol: String,
    /// Minimum conductance to ground (GMIN).
    pub(crate) gmin: String,
    /// Charge convergence tolerance (CHGTOL).
    pub(crate) chgtol: String,
    /// Absolute pivot tolerance for LU decomposition (PIVTOL).
    pub(crate) pivtol: String,
    /// Relative pivot tolerance (PIVREL).
    pub(crate) pivrel: String,

    // ── Output Control ─────────────────────────────────────────────────
    /// Number of significant digits in output (NUMDGT).
    pub(crate) numdgt: String,
}

impl Default for SimOptions {
    /// Default options matching standard SPICE defaults.
    fn default() -> Self {
        Self {
            temperature: "27".to_string(),
            tnom: "27".to_string(),
            method: "Trap".to_string(),
            itl1: "100".to_string(),
            itl2: "50".to_string(),
            itl4: "10".to_string(),
            itl5: "5000".to_string(),
            min_timestep: "0".to_string(),
            srcsteps: "0".to_string(),
            gminsteps: "0".to_string(),
            reltol: "0.001".to_string(),
            abstol: "1e-12".to_string(),
            vntol: "1e-6".to_string(),
            gmin: "1e-12".to_string(),
            chgtol: "1e-14".to_string(),
            pivtol: "1e-13".to_string(),
            pivrel: "1e-3".to_string(),
            numdgt: "6".to_string(),
        }
    }
}

impl SimOptions {
    /// Apply a named preset's values, overwriting all current settings.
    pub(crate) fn apply_preset(&mut self, preset: &str) {
        use osl_sim::{simulation_preset, SpiceMethod};
        let p = simulation_preset(preset);
        self.temperature = p.temperature;
        self.tnom = p.tnom;
        self.method = match p.method {
            SpiceMethod::Trap => "Trap".to_string(),
            SpiceMethod::Gear => "Gear".to_string(),
        };
        self.itl1 = p.itl1;
        self.itl2 = p.itl2;
        self.itl4 = p.itl4;
        self.itl5 = p.itl5;
        self.min_timestep = p.min_timestep;
        self.srcsteps = p.srcsteps;
        self.gminsteps = p.gminsteps;
        self.reltol = p.reltol;
        self.abstol = p.abstol;
        self.vntol = p.vntol;
        self.gmin = p.gmin;
        self.chgtol = p.chgtol;
        self.pivtol = p.pivtol;
        self.pivrel = p.pivrel;
        self.numdgt = p.numdgt;
    }
}
