//! TI/ADI SPICE 模型导入模块。
//!
//! 解析 Texas Instruments 和 Analog Devices 的 SPICE 模型库文件，
//! 提取子电路定义和模型参数，供仿真引擎使用。

use osl_core::{OslError, OslResult};
use std::path::{Path, PathBuf};
use std::collections::BTreeMap;

/// 厂商类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VendorKind {
    /// Texas Instruments
    Ti,
    /// Analog Devices / Linear Technology
    Adi,
    /// 通用（非厂商特定）
    Generic,
}

impl VendorKind {
    /// 根据文件路径推断厂商类型
    pub fn detect(path: &Path) -> Self {
        let path_str = path.to_string_lossy().to_lowercase();
        if path_str.contains("ti/") || path_str.contains("texas") || path_str.contains("ti_") {
            VendorKind::Ti
        } else if path_str.contains("adi/") || path_str.contains("analog") || path_str.contains("ltspice") || path_str.contains("lt_") {
            VendorKind::Adi
        } else {
            VendorKind::Generic
        }
    }

    /// 厂商名称
    pub fn name(self) -> &'static str {
        match self {
            VendorKind::Ti => "Texas Instruments",
            VendorKind::Adi => "Analog Devices",
            VendorKind::Generic => "Generic",
        }
    }
}

/// 导入的 SPICE 子电路
#[derive(Debug, Clone)]
pub struct ImportedSubckt {
    /// 子电路名称
    pub name: String,
    /// 引脚列表（按顺序）
    pub pins: Vec<String>,
    /// 原始子电路文本（含 .subckt 到 .ends）
    pub body: String,
    /// 源文件路径
    pub source_file: PathBuf,
    /// 行号
    pub line: usize,
    /// 厂商类型
    pub vendor: VendorKind,
}

/// 导入的 SPICE 模型
#[derive(Debug, Clone)]
pub struct ImportedModel {
    /// 模型名称
    pub name: String,
    /// 模型类型（NMOS, PMOS, NPN, PNP 等）
    pub model_type: String,
    /// 模型参数行
    pub params: String,
    /// 源文件路径
    pub source_file: PathBuf,
    /// 行号
    pub line: usize,
    /// 厂商类型
    pub vendor: VendorKind,
}

/// 厂商模型导入结果
#[derive(Debug)]
pub struct VendorImportResult {
    /// 检测到的厂商
    pub vendor: VendorKind,
    /// 导入的子电路
    pub subckts: Vec<ImportedSubckt>,
    /// 导入的模型
    pub models: Vec<ImportedModel>,
    /// 警告信息
    pub warnings: Vec<String>,
}

/// 从单个文件导入 SPICE 模型
pub fn import_spice_model_file(path: &Path) -> OslResult<VendorImportResult> {
    let content = osl_core::read_text(path)?;
    let vendor = VendorKind::detect(path);
    parse_spice_model_content(&content, path, vendor)
}

/// 从目录递归导入所有 SPICE 模型文件
pub fn import_spice_model_dir(root: &Path) -> OslResult<Vec<VendorImportResult>> {
    let mut results = Vec::new();
    import_dir_recursive(root, &mut results)?;
    Ok(results)
}

fn import_dir_recursive(path: &Path, results: &mut Vec<VendorImportResult>) -> OslResult<()> {
    if path.is_file() {
        if is_spice_model_file(path) {
            match import_spice_model_file(path) {
                Ok(result) => results.push(result),
                Err(_err) => {
                    // 跳过无法解析的文件，记录警告
                }
            }
        }
        return Ok(());
    }

    for entry in std::fs::read_dir(path)
        .map_err(|err| OslError::io(format!("read {}", path.display()), err))?
    {
        let entry = entry.map_err(|err| OslError::io("read directory entry", err))?;
        let entry_path = entry.path();
        if entry_path.is_dir() {
            import_dir_recursive(&entry_path, results)?;
        } else if is_spice_model_file(&entry_path) {
            match import_spice_model_file(&entry_path) {
                Ok(result) => results.push(result),
                Err(_) => {}
            }
        }
    }
    Ok(())
}

/// 判断文件是否为 SPICE 模型文件
pub fn is_spice_model_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "lib" | "mod" | "mdl" | "sub" | "subckt" | "sp" | "spice" | "cir"
            )
        })
}

/// 解析 SPICE 模型文件内容
fn parse_spice_model_content(
    content: &str,
    source_file: &Path,
    vendor: VendorKind,
) -> OslResult<VendorImportResult> {
    let mut result = VendorImportResult {
        vendor,
        subckts: Vec::new(),
        models: Vec::new(),
        warnings: Vec::new(),
    };

    let mut open_subckt: Option<SubcktBuilder> = None;
    let mut current_line = 0usize;

    for line in content.lines() {
        current_line += 1;
        let trimmed = line.trim();

        // 跳过空行和注释
        if trimmed.is_empty() || trimmed.starts_with('*') || trimmed.starts_with("//") {
            // 但如果在子电路内部，保留注释
            if let Some(ref mut builder) = open_subckt {
                builder.body.push_str(line);
                builder.body.push('\n');
            }
            continue;
        }

        // 处理续行（以 + 开头）
        if trimmed.starts_with('+') {
            if let Some(ref mut builder) = open_subckt {
                builder.body.push_str(line);
                builder.body.push('\n');
                // 解析续行中的引脚
                let continuation = &trimmed[1..];
                for token in continuation.split_whitespace() {
                    let token = token.trim_matches(',');
                    if !token.is_empty() && !token.contains('=') && !token.starts_with('.') {
                        builder.pending_pins.push(token.to_string());
                    }
                }
            }
            continue;
        }

        let upper = trimmed.to_uppercase();

        // .SUBCKT 开始
        if upper.starts_with(".SUBCKT") || upper.starts_with(".SUBCKT ") {
            if let Some(builder) = open_subckt.take() {
                result.warnings.push(format!(
                    "Nested .subckt at line {}: previous '{}' not closed with .ends",
                    current_line, builder.name
                ));
            }

            let tokens: Vec<&str> = trimmed.split_whitespace().collect();
            if tokens.len() < 2 {
                result.warnings.push(format!(
                    ".subckt missing name at line {}",
                    current_line
                ));
                continue;
            }

            let name = tokens[1].to_string();
            let pins: Vec<String> = tokens[2..]
                .iter()
                .take_while(|t| !t.contains('=') && !t.starts_with('.'))
                .map(|t| t.trim_matches(',').to_string())
                .filter(|t| !t.is_empty())
                .collect();

            open_subckt = Some(SubcktBuilder {
                name,
                pins,
                body: format!("{}\n", line),
                source_file: source_file.to_path_buf(),
                line: current_line,
                pending_pins: Vec::new(),
            });
            continue;
        }

        // .ENDS 关闭子电路
        if upper.starts_with(".ENDS") || upper.starts_with(".ENDSUBCKT") {
            if let Some(mut builder) = open_subckt.take() {
                builder.body.push_str(line);
                builder.body.push('\n');
                // 合并续行引脚
                builder.pins.extend(builder.pending_pins);

                result.subckts.push(ImportedSubckt {
                    name: builder.name,
                    pins: builder.pins,
                    body: builder.body,
                    source_file: builder.source_file,
                    line: builder.line,
                    vendor: result.vendor,
                });
            }
            continue;
        }

        // .MODEL 语句
        if upper.starts_with(".MODEL") || upper.starts_with(".MODEL ") {
            let tokens: Vec<&str> = trimmed.split_whitespace().collect();
            if tokens.len() >= 3 {
                result.models.push(ImportedModel {
                    name: tokens[1].to_string(),
                    model_type: tokens[2].to_string(),
                    params: trimmed.to_string(),
                    source_file: source_file.to_path_buf(),
                    line: current_line,
                    vendor: result.vendor,
                });
            }
        }

        // 如果在子电路内部，追加到 body
        if let Some(ref mut builder) = open_subckt {
            builder.body.push_str(line);
            builder.body.push('\n');
        }
    }

    // 检查未关闭的子电路
    if let Some(builder) = open_subckt {
        result.warnings.push(format!(
            "Unclosed .subckt '{}' starting at line {}",
            builder.name, builder.line
        ));
        // 仍然添加到结果中
        result.subckts.push(ImportedSubckt {
            name: builder.name,
            pins: builder.pins,
            body: builder.body,
            source_file: builder.source_file,
            line: builder.line,
            vendor: result.vendor,
        });
    }

    Ok(result)
}

/// 子电路构建器（解析过程中使用）
struct SubcktBuilder {
    name: String,
    pins: Vec<String>,
    body: String,
    source_file: PathBuf,
    line: usize,
    pending_pins: Vec<String>,
}

/// 将导入结果汇总为模型目录
pub fn build_model_catalog(results: &[VendorImportResult]) -> VendorModelCatalog {
    let mut catalog = VendorModelCatalog::default();

    for result in results {
        for subckt in &result.subckts {
            catalog.subckts.insert(
                subckt.name.clone(),
                ModelCatalogEntry {
                    name: subckt.name.clone(),
                    pins: subckt.pins.clone(),
                    source: subckt.source_file.display().to_string(),
                    vendor: subckt.vendor,
                },
            );
        }
        for model in &result.models {
            catalog.models.insert(
                model.name.clone(),
                ModelCatalogEntry {
                    name: model.name.clone(),
                    pins: Vec::new(),
                    source: model.source_file.display().to_string(),
                    vendor: model.vendor,
                },
            );
        }
    }

    catalog
}

/// 模型目录条目
#[derive(Debug, Clone)]
pub struct ModelCatalogEntry {
    pub name: String,
    pub pins: Vec<String>,
    pub source: String,
    pub vendor: VendorKind,
}

/// 厂商模型目录
#[derive(Debug, Default)]
pub struct VendorModelCatalog {
    pub subckts: BTreeMap<String, ModelCatalogEntry>,
    pub models: BTreeMap<String, ModelCatalogEntry>,
}

impl VendorModelCatalog {
    /// 总条目数
    pub fn total_count(&self) -> usize {
        self.subckts.len() + self.models.len()
    }

    /// 按厂商筛选
    pub fn filter_by_vendor(&self, vendor: VendorKind) -> Self {
        let subckts = self.subckts.iter()
            .filter(|(_, entry)| entry.vendor == vendor)
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let models = self.models.iter()
            .filter(|(_, entry)| entry.vendor == vendor)
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        Self { subckts, models }
    }

    /// 按名称搜索
    pub fn search(&self, query: &str) -> Self {
        let query = query.to_lowercase();
        let subckts = self.subckts.iter()
            .filter(|(name, _)| name.to_lowercase().contains(&query))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let models = self.models.iter()
            .filter(|(name, _)| name.to_lowercase().contains(&query))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        Self { subckts, models }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn imports_ti_subckt() {
        let content = r#"* TI TLV733P LDO Model
.SUBCKT TLV733P IN OUT GND EN
R1 IN OUT 0.1
C1 OUT GND 10p
.ends TLV733P

.MODEL NTLV733 NMOS (Vto=0.7 Kp=20u)
"#;
        let temp = std::env::temp_dir().join("nekospice_test_ti.lib");
        fs::write(&temp, content).unwrap();

        let result = import_spice_model_file(&temp).unwrap();
        assert_eq!(result.subckts.len(), 1);
        assert_eq!(result.subckts[0].name, "TLV733P");
        assert_eq!(result.subckts[0].pins, vec!["IN", "OUT", "GND", "EN"]);
        assert_eq!(result.models.len(), 1);

        fs::remove_file(&temp).unwrap();
    }

    #[test]
    fn imports_adi_subckt() {
        let content = r#"* ADI AD8065 SPICE Model
.SUBCKT AD8065 IN+ IN- OUT VCC VEE
R1 IN+ IN- 100k
E1 OUT 0 IN+ IN- 10
.ends AD8065

.MODEL MNP AD8065 NMOS (Vto=0.5)
"#;
        let temp = std::env::temp_dir().join("nekospice_test_adi.lib");
        fs::write(&temp, content).unwrap();

        let result = import_spice_model_file(&temp).unwrap();
        assert_eq!(result.subckts.len(), 1);
        assert_eq!(result.subckts[0].name, "AD8065");
        assert_eq!(result.subckts[0].pins.len(), 5);

        fs::remove_file(&temp).unwrap();
    }

    #[test]
    fn vendor_detection() {
        assert_eq!(VendorKind::detect(Path::new("/opt/ti/models/TLV733.lib")), VendorKind::Ti);
        assert_eq!(VendorKind::detect(Path::new("/opt/adi/models/AD8065.lib")), VendorKind::Adi);
        assert_eq!(VendorKind::detect(Path::new("/home/user/models/RC.lib")), VendorKind::Generic);
    }

    #[test]
    fn model_catalog_search() {
        let mut catalog = VendorModelCatalog::default();
        catalog.subckts.insert("TLV733P".to_string(), ModelCatalogEntry {
            name: "TLV733P".to_string(),
            pins: vec!["IN".into(), "OUT".into()],
            source: "ti.lib".into(),
            vendor: VendorKind::Ti,
        });
        catalog.subckts.insert("AD8065".to_string(), ModelCatalogEntry {
            name: "AD8065".to_string(),
            pins: vec!["IN+".into(), "IN-".into(), "OUT".into()],
            source: "adi.lib".into(),
            vendor: VendorKind::Adi,
        });

        let ti_only = catalog.filter_by_vendor(VendorKind::Ti);
        assert_eq!(ti_only.total_count(), 1);

        let search_result = catalog.search("TLV");
        assert_eq!(search_result.total_count(), 1);
    }
}
