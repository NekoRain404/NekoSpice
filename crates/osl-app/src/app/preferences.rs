//! 用户偏好管理。存储语言、主题、网格显示等界面设置。
//!
//! 偏好设置持久化到 `~/.config/nekospice/settings.json`。
//! 持久化结构定义见 [`preferences_persistence`]。

use super::NekoSpiceApp;
use super::localization::{StudioLocale, UiText};
use super::preferences_persistence::{SettingsFile, SimulationSettingsFile, settings_path};
use super::theme::{StudioPalette, StudioTheme, StudioThemeMode};
use std::fs;

/// 运行时偏好设置（从磁盘加载，内存中修改）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StudioPreferences {
    pub(super) theme_mode: StudioThemeMode,
    pub(super) locale: StudioLocale,
    /// ngspice 可执行文件路径
    pub(super) ngspice_path: String,
    /// Xyce 可执行文件路径
    pub(super) xyce_path: String,
}

impl Default for StudioPreferences {
    fn default() -> Self {
        Self::load_from_disk().unwrap_or_else(|| Self {
            theme_mode: StudioThemeMode::default(),
            locale: StudioLocale::default(),
            ngspice_path: "ngspice".to_string(),
            xyce_path: "xyce".to_string(),
        })
    }
}

impl StudioPreferences {
    /// 从磁盘加载偏好设置。
    fn load_from_disk() -> Option<Self> {
        let path = settings_path();
        let data = fs::read_to_string(&path).ok()?;
        let file: SettingsFile = serde_json::from_str(&data).ok()?;
        Some(Self {
            theme_mode: StudioThemeMode::from_str(&file.theme_mode),
            locale: StudioLocale::from_str(&file.locale),
            ngspice_path: file.ngspice_path,
            xyce_path: file.xyce_path,
        })
    }

    /// 保存当前偏好设置到磁盘（不含仿真选项）。
    pub(super) fn save_to_disk(&self) {
        let file = SettingsFile {
            theme_mode: self.theme_mode.as_str().to_string(),
            locale: self.locale.as_str().to_string(),
            ngspice_path: self.ngspice_path.clone(),
            xyce_path: self.xyce_path.clone(),
            simulation: SimulationSettingsFile::default(),
        };
        write_settings_file(&file);
    }

    /// 保存偏好设置到磁盘，同时包含仿真选项。
    ///
    /// 由 app 在仿真选项变更时调用，确保选项持久化。
    pub(super) fn save_with_simulation(
        &self,
        sim_opts: &super::simulation::sim_options::SimOptions,
        preset: &str,
        backend: &str,
        directive_kind: &str,
        toggles: &super::simulation::section_toggles::SimSectionToggles,
    ) {
        let file = SettingsFile {
            theme_mode: self.theme_mode.as_str().to_string(),
            locale: self.locale.as_str().to_string(),
            ngspice_path: self.ngspice_path.clone(),
            xyce_path: self.xyce_path.clone(),
            simulation: SimulationSettingsFile {
                temperature: sim_opts.temperature.clone(),
                tnom: sim_opts.tnom.clone(),
                method: sim_opts.method.clone(),
                itl1: sim_opts.itl1.clone(),
                itl2: sim_opts.itl2.clone(),
                itl4: sim_opts.itl4.clone(),
                itl5: sim_opts.itl5.clone(),
                min_timestep: sim_opts.min_timestep.clone(),
                srcsteps: sim_opts.srcsteps.clone(),
                gminsteps: sim_opts.gminsteps.clone(),
                reltol: sim_opts.reltol.clone(),
                abstol: sim_opts.abstol.clone(),
                vntol: sim_opts.vntol.clone(),
                gmin: sim_opts.gmin.clone(),
                chgtol: sim_opts.chgtol.clone(),
                pivtol: sim_opts.pivtol.clone(),
                pivrel: sim_opts.pivrel.clone(),
                numdgt: sim_opts.numdgt.clone(),
                active_preset: preset.to_string(),
                backend: backend.to_string(),
                directive_kind: directive_kind.to_string(),
                section_toggles: super::preferences_persistence::SectionTogglesFile {
                    step_sweep: toggles.step_sweep,
                    measurements: toggles.measurements,
                    initial_conditions: toggles.initial_conditions,
                    component_params: toggles.component_params,
                    model_params: toggles.model_params,
                    quick_start: toggles.quick_start,
                    netlist_preview: toggles.netlist_preview,
                    run_status: toggles.run_status,
                    transient_solver: toggles.transient_solver,
                    convergence: toggles.convergence,
                    output_control: toggles.output_control,
                },
            },
        };
        write_settings_file(&file);
    }

    /// 从磁盘加载仿真选项（如果存在）。
    /// 返回 (SimOptions, active_preset_name, backend, directive_kind, section_toggles)。
    pub(super) fn load_simulation_settings() -> (
        super::simulation::sim_options::SimOptions,
        String,
        String,
        String,
        super::simulation::section_toggles::SimSectionToggles,
    ) {
        let path = settings_path();
        let data = match fs::read_to_string(&path) {
            Ok(d) => d,
            Err(_) => {
                return (
                    super::simulation::sim_options::SimOptions::default(),
                    "default".to_string(),
                    "ngspice".to_string(),
                    "tran".to_string(),
                    super::simulation::section_toggles::SimSectionToggles::default(),
                );
            }
        };
        let file: SettingsFile = match serde_json::from_str(&data) {
            Ok(f) => f,
            Err(_) => {
                return (
                    super::simulation::sim_options::SimOptions::default(),
                    "default".to_string(),
                    "ngspice".to_string(),
                    "tran".to_string(),
                    super::simulation::section_toggles::SimSectionToggles::default(),
                );
            }
        };
        let s = file.simulation;
        let opts = super::simulation::sim_options::SimOptions {
            temperature: s.temperature,
            tnom: s.tnom,
            method: s.method,
            itl1: s.itl1,
            itl2: s.itl2,
            itl4: s.itl4,
            itl5: s.itl5,
            min_timestep: s.min_timestep,
            srcsteps: s.srcsteps,
            gminsteps: s.gminsteps,
            reltol: s.reltol,
            abstol: s.abstol,
            vntol: s.vntol,
            gmin: s.gmin,
            chgtol: s.chgtol,
            pivtol: s.pivtol,
            pivrel: s.pivrel,
            numdgt: s.numdgt,
        };
        let toggles = super::simulation::section_toggles::SimSectionToggles {
            step_sweep: s.section_toggles.step_sweep,
            measurements: s.section_toggles.measurements,
            initial_conditions: s.section_toggles.initial_conditions,
            component_params: s.section_toggles.component_params,
            model_params: s.section_toggles.model_params,
            quick_start: s.section_toggles.quick_start,
            netlist_preview: s.section_toggles.netlist_preview,
            run_status: s.section_toggles.run_status,
            transient_solver: s.section_toggles.transient_solver,
            convergence: s.section_toggles.convergence,
            output_control: s.section_toggles.output_control,
        };
        (opts, s.active_preset, s.backend, s.directive_kind, toggles)
    }
}

/// 将 SettingsFile 序列化并写入磁盘。
fn write_settings_file(file: &SettingsFile) {
    let path = settings_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(file) {
        let _ = fs::write(&path, json);
    }
}

impl NekoSpiceApp {
    /// 当前主题模式
    pub(super) fn theme_mode(&self) -> StudioThemeMode {
        self.preferences.theme_mode
    }

    /// 当前语言
    pub(super) fn locale(&self) -> StudioLocale {
        self.preferences.locale
    }

    /// 根据当前语言获取 UI 文本
    pub(super) fn text(&self, key: UiText) -> &'static str {
        self.locale().text(key)
    }

    /// 获取当前主题调色板
    pub(super) fn theme_palette(&self) -> StudioPalette {
        StudioTheme::palette(self.theme_mode())
    }

    /// 切换主题模式并保存
    pub(super) fn toggle_theme_mode(&mut self) {
        self.preferences.theme_mode = self.preferences.theme_mode.next();
        self.preferences.save_to_disk();
    }

    /// 切换语言并保存
    pub(super) fn toggle_locale(&mut self) {
        self.preferences.locale = self.preferences.locale.next();
        self.preferences.save_to_disk();
    }

    /// 保存仿真选项到磁盘。
    /// 在仿真选项变更时由 UI 调用。
    pub(super) fn save_simulation_settings(&self) {
        self.preferences.save_with_simulation(
            &self.simulation_profile_editor.options,
            &self.simulation_profile_editor.active_preset,
            self.simulation_panel.backend.label(),
            &self.simulation_panel.directive_kind.to_string(),
            &self.simulation_profile_editor.toggles,
        );
    }

    /// 主题模式显示文本
    pub(super) fn theme_mode_label(&self, mode: StudioThemeMode) -> &'static str {
        match self.locale() {
            StudioLocale::English => mode.label(),
            StudioLocale::SimplifiedChinese => mode.label_zh(),
        }
    }
}
