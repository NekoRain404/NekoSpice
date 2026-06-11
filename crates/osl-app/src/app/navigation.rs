use super::localization::StudioLocale;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub(super) enum StudioWorkspace {
    #[default]
    Home,
    Schematic,
    Library,
    Simulation,
    Optimization,
    Review,
    Waveforms,
    Reports,
    Settings,
}

impl StudioWorkspace {
    pub(super) const ALL: [Self; 9] = [
        Self::Home,
        Self::Schematic,
        Self::Library,
        Self::Simulation,
        Self::Optimization,
        Self::Review,
        Self::Waveforms,
        Self::Reports,
        Self::Settings,
    ];

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Home => "Home",
            Self::Schematic => "Schematic",
            Self::Library => "Library",
            Self::Simulation => "Simulation",
            Self::Optimization => "Optimize",
            Self::Review => "Review",
            Self::Waveforms => "Waveforms",
            Self::Reports => "Reports",
            Self::Settings => "Settings",
        }
    }

    pub(super) fn localized_label(self, locale: StudioLocale) -> &'static str {
        match locale {
            StudioLocale::English => self.label(),
            StudioLocale::SimplifiedChinese => match self {
                Self::Home => "首页",
                Self::Schematic => "原理图",
                Self::Library => "符号库",
                Self::Simulation => "仿真",
                Self::Optimization => "优化",
                Self::Review => "审查",
                Self::Waveforms => "波形",
                Self::Reports => "报告",
                Self::Settings => "设置",
            },
        }
    }

    #[allow(dead_code)]
    pub(super) fn icon(self) -> &'static str {
        match self {
            Self::Home => "HME",
            Self::Schematic => "SCH",
            Self::Library => "LIB",
            Self::Simulation => "SIM",
            Self::Optimization => "OPT",
            Self::Review => "REV",
            Self::Waveforms => "WAV",
            Self::Reports => "RPT",
            Self::Settings => "SET",
        }
    }

    #[allow(dead_code)]
    pub(super) fn caption(self) -> &'static str {
        match self {
            Self::Home => "Project dashboard and engineering shortcuts",
            Self::Schematic => "Edit KiCad-compatible sheets",
            Self::Library => "Browse symbols and placement scope",
            Self::Simulation => "Run ngspice and inspect outputs",
            Self::Optimization => "Tune parameters, sweeps, and yield",
            Self::Review => "Rank schematic risks and recommended fixes",
            Self::Waveforms => "Analyze waveform traces and measurements",
            Self::Reports => "Review artifacts, waveforms, reports",
            Self::Settings => "Configure theme and language",
        }
    }

    #[allow(dead_code)]
    pub(super) fn localized_caption(self, locale: StudioLocale) -> &'static str {
        match locale {
            StudioLocale::English => self.caption(),
            StudioLocale::SimplifiedChinese => match self {
                Self::Home => "项目看板和工程快捷入口",
                Self::Schematic => "编辑兼容 KiCad 的图纸",
                Self::Library => "浏览符号和放置范围",
                Self::Simulation => "运行 ngspice 并检查输出",
                Self::Optimization => "调优参数、扫描和良率",
                Self::Review => "排序原理图风险和推荐修复",
                Self::Waveforms => "分析波形轨迹和测量结果",
                Self::Reports => "查看产物、波形和报告",
                Self::Settings => "配置主题和语言",
            },
        }
    }

    pub(super) fn from_slug(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "home" => Some(Self::Home),
            "schematic" | "sch" => Some(Self::Schematic),
            "library" | "lib" => Some(Self::Library),
            "simulation" | "sim" => Some(Self::Simulation),
            "optimization" | "optimize" | "opt" | "sweep" | "montecarlo" => {
                Some(Self::Optimization)
            }
            "review" | "design-review" | "drc" | "audit" => Some(Self::Review),
            "waveforms" | "waveform" | "waves" | "wav" | "analysis" => Some(Self::Waveforms),
            "reports" | "report" | "rpt" => Some(Self::Reports),
            "settings" | "set" => Some(Self::Settings),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::StudioLocale;
    use super::StudioWorkspace;

    #[test]
    fn studio_workspaces_have_stable_labels() {
        assert_eq!(StudioWorkspace::ALL.len(), 9);
        assert_eq!(StudioWorkspace::default(), StudioWorkspace::Home);
        assert_eq!(StudioWorkspace::Schematic.label(), "Schematic");
        assert_eq!(
            StudioWorkspace::Schematic.localized_label(StudioLocale::SimplifiedChinese),
            "原理图"
        );
        assert_eq!(
            StudioWorkspace::Waveforms.localized_label(StudioLocale::SimplifiedChinese),
            "波形"
        );
        assert!(!StudioWorkspace::Simulation.caption().is_empty());
        assert_eq!(
            StudioWorkspace::from_slug("sch"),
            Some(StudioWorkspace::Schematic)
        );
        assert_eq!(
            StudioWorkspace::from_slug("analysis"),
            Some(StudioWorkspace::Waveforms)
        );
        assert_eq!(
            StudioWorkspace::from_slug("sweep"),
            Some(StudioWorkspace::Optimization)
        );
        assert_eq!(
            StudioWorkspace::from_slug("audit"),
            Some(StudioWorkspace::Review)
        );
        assert_eq!(StudioWorkspace::from_slug("unknown"), None);
    }
}
