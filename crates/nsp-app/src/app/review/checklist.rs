//! Design review checklist — categorized pass/fail items with progress tracking.

use super::state::ReviewChecklistTab;
use crate::app::NekoSpiceApp;
use crate::app::localization::{StudioLocale, UiText};
use crate::app::navigation::StudioWorkspace;
use crate::app::theme::{StudioTheme, StudioThemeMode};
use eframe::egui::{self, Color32, RichText};

impl NekoSpiceApp {
    /// draw review checklist board。
    pub(crate) fn draw_review_checklist_board(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let locale = self.locale();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::DesignChecklist),
            ));
            ui.horizontal_wrapped(|ui| {
                for tab in ReviewChecklistTab::ALL {
                    ui.selectable_value(
                        &mut self.review_workspace.checklist_tab,
                        tab,
                        tab.label(locale),
                    );
                }
            });
            ui.add_space(6.0);
            self.draw_checklist_progress(ui);
            ui.separator();

            for item in REVIEW_CHECKLIST {
                if item.tab == self.review_workspace.checklist_tab {
                    checklist_item_row(ui, mode, locale, item);
                }
            }

            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                if ui.button(self.text(UiText::RunSimulation)).clicked() {
                    self.active_workspace = StudioWorkspace::Simulation;
                }
                if ui.button(self.text(UiText::OpenSchematic)).clicked() {
                    self.active_workspace = StudioWorkspace::Schematic;
                }
                if ui.button(self.text(UiText::FindOptimization)).clicked() {
                    self.active_workspace = StudioWorkspace::Optimization;
                }
            });
        });
    }

    fn draw_checklist_progress(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let tab = self.review_workspace.checklist_tab;
        let total = REVIEW_CHECKLIST
            .iter()
            .filter(|item| item.tab == tab)
            .count()
            .max(1);
        let passed = REVIEW_CHECKLIST
            .iter()
            .filter(|item| item.tab == tab && item.status == ChecklistStatus::Pass)
            .count();
        let ratio = passed as f32 / total as f32;
        ui.horizontal(|ui| {
            ui.label(StudioTheme::muted_for(
                mode,
                format!("{}: {passed}/{total}", self.text(UiText::Verified)),
            ));
            ui.add(
                egui::ProgressBar::new(ratio)
                    .desired_width(ui.available_width())
                    .show_percentage(),
            );
        });
    }
}

#[derive(Debug, Clone, Copy)]
struct ChecklistItem {
    tab: ReviewChecklistTab,
    status: ChecklistStatus,
    title_en: &'static str,
    title_zh: &'static str,
    detail_en: &'static str,
    detail_zh: &'static str,
    margin: &'static str,
}

impl ChecklistItem {
    fn title(self, locale: StudioLocale) -> &'static str {
        match locale {
            StudioLocale::English => self.title_en,
            StudioLocale::SimplifiedChinese => self.title_zh,
        }
    }

    fn detail(self, locale: StudioLocale) -> &'static str {
        match locale {
            StudioLocale::English => self.detail_en,
            StudioLocale::SimplifiedChinese => self.detail_zh,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChecklistStatus {
    Pass,
    Review,
    Blocked,
}

impl ChecklistStatus {
    fn label(self, locale: StudioLocale) -> &'static str {
        match locale {
            StudioLocale::English => match self {
                Self::Pass => "Pass",
                Self::Review => "Review",
                Self::Blocked => "Blocked",
            },
            StudioLocale::SimplifiedChinese => match self {
                Self::Pass => "通过",
                Self::Review => "复查",
                Self::Blocked => "阻塞",
            },
        }
    }

    fn color(self, mode: StudioThemeMode) -> Color32 {
        let palette = StudioTheme::palette(mode);
        match self {
            Self::Pass => palette.success,
            Self::Review => palette.warning,
            Self::Blocked => palette.danger,
        }
    }
}

// Seed checklist data mirrors the reference UI while the rules engine matures.
const REVIEW_CHECKLIST: [ChecklistItem; 9] = [
    ChecklistItem {
        tab: ReviewChecklistTab::Readiness,
        status: ChecklistStatus::Pass,
        title_en: "Schematic parses as schema document",
        title_zh: "原理图可解析为 schema 文档",
        detail_en: "Rust IR loaded, canvas scene available",
        detail_zh: "Rust IR 已加载，画布场景可用",
        margin: "100%",
    },
    ChecklistItem {
        tab: ReviewChecklistTab::Readiness,
        status: ChecklistStatus::Review,
        title_en: "Simulation directive coverage",
        title_zh: "仿真指令覆盖",
        detail_en: "Transient profile exists; AC profile recommended",
        detail_zh: "已有瞬态配置，建议补充 AC 配置",
        margin: "67%",
    },
    ChecklistItem {
        tab: ReviewChecklistTab::Readiness,
        status: ChecklistStatus::Pass,
        title_en: "Library references resolved",
        title_zh: "库引用已解析",
        detail_en: "Project symbol table linked",
        detail_zh: "项目符号表已联动",
        margin: "100%",
    },
    ChecklistItem {
        tab: ReviewChecklistTab::Electrical,
        status: ChecklistStatus::Blocked,
        title_en: "Output headroom",
        title_zh: "输出摆幅余量",
        detail_en: "Output approaches supply rail in review model",
        detail_zh: "审查模型中输出接近电源轨",
        margin: "-3.2 dB",
    },
    ChecklistItem {
        tab: ReviewChecklistTab::Electrical,
        status: ChecklistStatus::Review,
        title_en: "Phase margin target",
        title_zh: "相位裕量目标",
        detail_en: "Compensation should be verified against load",
        detail_zh: "补偿网络需要结合负载验证",
        margin: "58 deg",
    },
    ChecklistItem {
        tab: ReviewChecklistTab::Electrical,
        status: ChecklistStatus::Pass,
        title_en: "Power decoupling present",
        title_zh: "存在电源去耦",
        detail_en: "Local bypass strategy detected",
        detail_zh: "检测到本地旁路策略",
        margin: "OK",
    },
    ChecklistItem {
        tab: ReviewChecklistTab::Models,
        status: ChecklistStatus::Pass,
        title_en: "SPICE pin mapping",
        title_zh: "SPICE 引脚映射",
        detail_en: "Symbol pins match subcircuit order",
        detail_zh: "符号引脚匹配子电路顺序",
        margin: "100%",
    },
    ChecklistItem {
        tab: ReviewChecklistTab::Models,
        status: ChecklistStatus::Review,
        title_en: "Vendor model freshness",
        title_zh: "厂商模型新鲜度",
        detail_en: "One update is available in the model library",
        detail_zh: "模型库中有一个可用更新",
        margin: "1",
    },
    ChecklistItem {
        tab: ReviewChecklistTab::Models,
        status: ChecklistStatus::Pass,
        title_en: "Ngspice compatibility",
        title_zh: "Ngspice 兼容性",
        detail_en: "No unsupported control statements detected",
        detail_zh: "未发现不支持的控制语句",
        margin: "OK",
    },
];

fn checklist_item_row(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    locale: StudioLocale,
    item: ChecklistItem,
) {
    ui.horizontal(|ui| {
        let color = item.status.color(mode);
        let row_width = ui.available_width();
        let status_width = 64.0;
        let margin_width = 112.0;
        let text_width = (row_width - status_width - margin_width - 18.0).max(160.0);

        ui.allocate_ui_with_layout(
            egui::vec2(status_width, 42.0),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.label(
                    RichText::new(item.status.label(locale))
                        .strong()
                        .color(color),
                );
            },
        );
        ui.allocate_ui_with_layout(
            egui::vec2(text_width, 42.0),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.label(RichText::new(item.title(locale)).strong());
                ui.label(StudioTheme::muted_for(mode, item.detail(locale)));
            },
        );
        ui.allocate_ui_with_layout(
            egui::vec2(margin_width, 42.0),
            egui::Layout::right_to_left(egui::Align::Center),
            |ui| {
                ui.label(RichText::new(item.margin).color(color));
                ui.label(StudioTheme::muted_for(mode, "Margin"));
            },
        );
    });
    ui.separator();
}
