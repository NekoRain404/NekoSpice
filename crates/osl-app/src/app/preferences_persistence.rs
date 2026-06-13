//! 偏好设置持久化层。定义磁盘上的 JSON 结构和路径工具。
//!
//! 持久化内容包括：
//! - UI 偏好（主题、语言）
//! - 求解器路径（ngspice、xyce）
//! - 仿真设置（温度、容差、迭代限制等）

use std::path::PathBuf;

/// 持久化到磁盘的偏好设置 JSON 结构。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(super) struct SettingsFile {
    pub(super) theme_mode: String,
    pub(super) locale: String,
    pub(super) ngspice_path: String,
    pub(super) xyce_path: String,
    /// 仿真设置（可选，旧版本配置文件可能不含此字段）。
    #[serde(default)]
    pub(super) simulation: SimulationSettingsFile,
}

/// 仿真设置的持久化表示。
/// 使用 serde(default) 保证向后兼容：旧配置文件不含此字段时使用默认值。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(super) struct SimulationSettingsFile {
    pub(super) temperature: String,
    pub(super) tnom: String,
    pub(super) method: String,
    pub(super) itl1: String,
    pub(super) itl2: String,
    pub(super) itl4: String,
    pub(super) itl5: String,
    pub(super) min_timestep: String,
    pub(super) srcsteps: String,
    pub(super) gminsteps: String,
    pub(super) reltol: String,
    pub(super) abstol: String,
    pub(super) vntol: String,
    pub(super) gmin: String,
    pub(super) chgtol: String,
    pub(super) pivtol: String,
    pub(super) pivrel: String,
    pub(super) numdgt: String,
    pub(super) active_preset: String,
    #[serde(default)]
    pub(super) backend: String,
    #[serde(default)]
    pub(super) directive_kind: String,
    /// Section visibility toggles for the profile editor.
    #[serde(default = "default_section_toggles")]
    pub(super) section_toggles: SectionTogglesFile,
}

/// Persisted section toggle states.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(super) struct SectionTogglesFile {
    pub step_sweep: bool,
    pub measurements: bool,
    pub initial_conditions: bool,
    pub component_params: bool,
    pub model_params: bool,
    pub quick_start: bool,
    pub netlist_preview: bool,
    pub run_status: bool,
    pub transient_solver: bool,
    pub convergence: bool,
    pub output_control: bool,
}

fn default_section_toggles() -> SectionTogglesFile {
    SectionTogglesFile::default()
}

impl Default for SectionTogglesFile {
    fn default() -> Self {
        Self {
            step_sweep: true,
            measurements: true,
            initial_conditions: false,
            component_params: true,
            model_params: false,
            quick_start: true,
            netlist_preview: true,
            run_status: true,
            transient_solver: false,
            convergence: false,
            output_control: true,
        }
    }
}

impl Default for SimulationSettingsFile {
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
            active_preset: "default".to_string(),
            backend: "ngspice".to_string(),
            directive_kind: "tran".to_string(),
            section_toggles: SectionTogglesFile::default(),
        }
    }
}

impl Default for SettingsFile {
    fn default() -> Self {
        Self {
            theme_mode: "Dark".to_string(),
            locale: "en".to_string(),
            ngspice_path: "ngspice".to_string(),
            xyce_path: "xyce".to_string(),
            simulation: SimulationSettingsFile::default(),
        }
    }
}

/// 获取设置文件路径：`~/.config/nekospice/settings.json`
pub(super) fn settings_path() -> PathBuf {
    dirs_or_fallback()
        .join("nekospice")
        .join("settings.json")
}

/// 优先使用 XDG_CONFIG_HOME，回退到 HOME/.config
fn dirs_or_fallback() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var_os("HOME")
                .map(|home| PathBuf::from(home).join(".config"))
                .unwrap_or_else(|| PathBuf::from("."))
        })
}
