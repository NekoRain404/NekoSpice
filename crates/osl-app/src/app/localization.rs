#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum StudioLocale {
    #[default]
    English,
    SimplifiedChinese,
}

impl StudioLocale {
    pub(super) const ALL: [Self; 2] = [Self::English, Self::SimplifiedChinese];

    pub(super) fn native_name(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::SimplifiedChinese => "简体中文",
        }
    }

    pub(super) fn short_code(self) -> &'static str {
        match self {
            Self::English => "EN",
            Self::SimplifiedChinese => "中",
        }
    }

    pub(super) fn next(self) -> Self {
        match self {
            Self::English => Self::SimplifiedChinese,
            Self::SimplifiedChinese => Self::English,
        }
    }

    pub(super) fn text(self, key: UiText) -> &'static str {
        match self {
            Self::English => key.en(),
            Self::SimplifiedChinese => key.zh_hans(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum UiText {
    ActiveProject,
    Appearance,
    Buses,
    Clean,
    CurrentLanguage,
    CurrentTheme,
    DeleteSelected,
    Diagnostics,
    Dirty,
    Document,
    EditCommands,
    Fit,
    FitHint,
    Graphics,
    Info,
    Kind,
    Labels,
    Language,
    Missing,
    NoDiagnostics,
    NoDocument,
    NoEditableSchematicLoaded,
    NoProject,
    NoSchematicLoaded,
    NoSelectedItem,
    NoSelection,
    NoWaveform,
    OpenSchematic,
    Project,
    Renderer,
    ReportsCaption,
    ReportsResults,
    Run,
    RunHint,
    Saved,
    Save,
    SaveHint,
    SchematicHealth,
    SchematicTools,
    SchematicToolsCaption,
    Selection,
    Settings,
    Sheets,
    Solver,
    StudioSubtitle,
    StudioTitle,
    Symbols,
    System,
    Theme,
    UnsavedChanges,
    WaveformError,
    Wires,
    Workspace,
    Zoom,
}

impl UiText {
    fn en(self) -> &'static str {
        match self {
            Self::ActiveProject => "Active Project",
            Self::Appearance => "Appearance",
            Self::Buses => "Buses",
            Self::Clean => "clean",
            Self::CurrentLanguage => "Current language",
            Self::CurrentTheme => "Current theme",
            Self::DeleteSelected => "Delete Selected",
            Self::Diagnostics => "Diagnostics",
            Self::Dirty => "dirty",
            Self::Document => "Document",
            Self::EditCommands => "Edit Commands",
            Self::Fit => "Fit",
            Self::FitHint => "Fit the schematic to the canvas",
            Self::Graphics => "Graphics",
            Self::Info => "info",
            Self::Kind => "Kind",
            Self::Labels => "Labels",
            Self::Language => "Language",
            Self::Missing => "missing",
            Self::NoDiagnostics => "No diagnostics",
            Self::NoDocument => "No document",
            Self::NoEditableSchematicLoaded => "No editable schematic loaded",
            Self::NoProject => "No project",
            Self::NoSchematicLoaded => "No schematic loaded",
            Self::NoSelectedItem => "No selected item",
            Self::NoSelection => "No selection",
            Self::NoWaveform => "No waveform",
            Self::OpenSchematic => "Open Schematic",
            Self::Project => "Project",
            Self::Renderer => "Renderer",
            Self::ReportsCaption => {
                "Latest run artifacts, generated HTML reports, and waveform previews."
            }
            Self::ReportsResults => "Reports & Results",
            Self::Run => "Run",
            Self::RunHint => "Run ngspice for the active schematic",
            Self::Saved => "Saved",
            Self::Save => "Save",
            Self::SaveHint => "Save the active KiCad schematic",
            Self::SchematicHealth => "Schematic Health",
            Self::SchematicTools => "Schematic Tools",
            Self::SchematicToolsCaption => "Place wires, labels, buses, sheets, and markers.",
            Self::Selection => "Selection",
            Self::Settings => "Settings",
            Self::Sheets => "Sheets",
            Self::Solver => "Solver",
            Self::StudioSubtitle => "Rust-native KiCad schematic and ngspice studio",
            Self::StudioTitle => "NekoSpice Studio",
            Self::Symbols => "Symbols",
            Self::System => "System",
            Self::Theme => "Theme",
            Self::UnsavedChanges => "Unsaved changes",
            Self::WaveformError => "Waveform error",
            Self::Wires => "Wires",
            Self::Workspace => "Workspace",
            Self::Zoom => "Zoom",
        }
    }

    fn zh_hans(self) -> &'static str {
        match self {
            Self::ActiveProject => "当前项目",
            Self::Appearance => "外观",
            Self::Buses => "总线",
            Self::Clean => "已保存",
            Self::CurrentLanguage => "当前语言",
            Self::CurrentTheme => "当前主题",
            Self::DeleteSelected => "删除选中项",
            Self::Diagnostics => "诊断",
            Self::Dirty => "未保存",
            Self::Document => "文档",
            Self::EditCommands => "编辑命令",
            Self::Fit => "适配",
            Self::FitHint => "将原理图适配到画布",
            Self::Graphics => "图形",
            Self::Info => "信息",
            Self::Kind => "类型",
            Self::Labels => "标签",
            Self::Language => "语言",
            Self::Missing => "缺失",
            Self::NoDiagnostics => "无诊断",
            Self::NoDocument => "无文档",
            Self::NoEditableSchematicLoaded => "未加载可编辑原理图",
            Self::NoProject => "无项目",
            Self::NoSchematicLoaded => "未加载原理图",
            Self::NoSelectedItem => "未选择项目",
            Self::NoSelection => "未选择",
            Self::NoWaveform => "无波形",
            Self::OpenSchematic => "打开原理图",
            Self::Project => "项目",
            Self::Renderer => "渲染器",
            Self::ReportsCaption => "最近运行产物、HTML 报告和波形预览。",
            Self::ReportsResults => "报告与结果",
            Self::Run => "运行",
            Self::RunHint => "使用 ngspice 运行当前原理图",
            Self::Saved => "已保存",
            Self::Save => "保存",
            Self::SaveHint => "保存当前 KiCad 原理图",
            Self::SchematicHealth => "原理图概况",
            Self::SchematicTools => "原理图工具",
            Self::SchematicToolsCaption => "放置导线、标签、总线、层级图纸和标记。",
            Self::Selection => "选择",
            Self::Settings => "设置",
            Self::Sheets => "图纸",
            Self::Solver => "求解器",
            Self::StudioSubtitle => "Rust 原生 KiCad 原理图与 ngspice 工作台",
            Self::StudioTitle => "NekoSpice 工作台",
            Self::Symbols => "符号",
            Self::System => "系统",
            Self::Theme => "主题",
            Self::UnsavedChanges => "未保存更改",
            Self::WaveformError => "波形错误",
            Self::Wires => "导线",
            Self::Workspace => "工作区",
            Self::Zoom => "缩放",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{StudioLocale, UiText};

    #[test]
    fn locales_have_stable_labels() {
        assert_eq!(StudioLocale::ALL.len(), 2);
        assert_eq!(StudioLocale::English.text(UiText::Settings), "Settings");
        assert_eq!(
            StudioLocale::SimplifiedChinese.text(UiText::Settings),
            "设置"
        );
    }
}
