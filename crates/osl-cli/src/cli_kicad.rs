// KiCad CLI commands.
// Covers: kicad_inspect, kicad_select, kicad_check,
// kicad_export, kicad_edit, kicad_render, copy_import_dependencies.

fn kicad_inspect_command(args: &[String]) -> OslResult<i32> {
    let input = positional(args, 0, "missing KiCad path for 'osl kicad-inspect'")?;
    let output = flag_value(args, "--output");
    let path = Path::new(input);
    let should_emit_canvas = has_flag(args, "--canvas");
    let should_index = has_flag(args, "--index");
    let index_query = KicadSymbolLibraryIndexQuery {
        text: flag_value(args, "--query"),
        library: flag_value(args, "--library"),
        footprint: flag_value(args, "--footprint"),
    };
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();

    let json = match (
        extension.as_str(),
        path.file_name().and_then(|name| name.to_str()),
    ) {
        ("kicad_sch", _) if should_emit_canvas => read_kicad_schematic_with_libraries(path)?
            .canvas_scene()
            .to_json(),
        ("kicad_sch", _) => read_kicad_schematic_with_libraries(path)?.to_summary_json(),
        ("kicad_pro", _) => read_kicad_project(path)?.to_summary_json(),
        ("kicad_sym", _) => read_kicad_symbol_library(path)?.to_summary_json(),
        (_, Some("sym-lib-table")) if should_index => {
            let index = read_kicad_symbol_library_index(path)?;
            if index_query.is_empty() {
                index.to_json()
            } else {
                index.query(&index_query).to_json()
            }
        }
        (_, Some("sym-lib-table")) => read_kicad_symbol_library_table(path)?.to_summary_json(),
        _ => {
            return Err(OslError::InvalidInput(format!(
                "{} is not a supported KiCad project/schematic/library file (.kicad_pro, .kicad_sch, .kicad_sym, sym-lib-table)",
                path.display()
            )));
        }
    };

    if let Some(output) = output {
        write_text(Path::new(&output), &json)?;
        println!("kicad-inspect -> {output}");
    } else {
        print!("{json}");
    }
    Ok(0)
}

fn kicad_select_command(args: &[String]) -> OslResult<i32> {
    let input = positional(args, 0, "missing KiCad path for 'osl kicad-select'")?;
    let point = positional(args, 1, "missing point for 'osl kicad-select'")?;
    let output = flag_value(args, "--output");
    let input_path = Path::new(input);
    let extension = input_path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    if extension != "kicad_sch" {
        return Err(OslError::InvalidInput(format!(
            "{} is not a supported KiCad select input (.kicad_sch)",
            input_path.display()
        )));
    }

    let point = parse_kicad_point(point, "KiCad canvas select point")?;
    let json = read_kicad_schematic_with_libraries(input_path)?
        .canvas_scene()
        .hit_test(point)
        .to_json();

    if let Some(output) = output {
        write_text(Path::new(&output), &json)?;
        println!("kicad-select -> {output}");
    } else {
        print!("{json}");
    }
    Ok(0)
}

fn kicad_check_command(args: &[String]) -> OslResult<i32> {
    let input = positional(args, 0, "missing KiCad path for 'osl kicad-check'")?;
    let output = flag_value(args, "--output");
    let input_path = Path::new(input);
    let extension = input_path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    if extension != "kicad_sch" {
        return Err(OslError::InvalidInput(format!(
            "{} is not a supported KiCad check input (.kicad_sch)",
            input_path.display()
        )));
    }

    let schematic = read_kicad_schematic_with_libraries(input_path)?;
    let report = schematic
        .check_report_with_hierarchy(input_path.parent().unwrap_or_else(|| Path::new(".")))?;
    let json = report.to_json();
    if let Some(output) = output {
        write_text(Path::new(&output), &json)?;
        println!(
            "kicad-check: {} diagnostics ({} errors, {} warnings) -> {}",
            report.diagnostics.len(),
            report.error_count(),
            report.warning_count(),
            output
        );
    } else {
        print!("{json}");
    }

    Ok(if report.error_count() == 0 { 0 } else { 2 })
}

fn kicad_export_command(args: &[String]) -> OslResult<i32> {
    let input = positional(args, 0, "missing KiCad path for 'osl kicad-export'")?;
    let output = flag_value(args, "--output").ok_or_else(|| {
        OslError::InvalidInput("missing --output <file> for 'osl kicad-export'".to_string())
    })?;
    let input_path = Path::new(input);
    let output_path = Path::new(&output);
    let extension = input_path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();

    match extension.as_str() {
        "kicad_sch" => {
            let schematic = read_kicad_schematic_with_libraries(input_path)?;
            write_kicad_schematic(output_path, &schematic)?;
        }
        "kicad_sym" => {
            let library = read_kicad_symbol_library(input_path)?;
            write_kicad_symbol_library(output_path, &library)?;
        }
        _ => {
            return Err(OslError::InvalidInput(format!(
                "{} is not a supported KiCad export input (.kicad_sch, .kicad_sym)",
                input_path.display()
            )));
        }
    }

    println!("kicad-export -> {output}");
    Ok(0)
}

fn kicad_edit_command(args: &[String]) -> OslResult<i32> {
    let input = positional(args, 0, "missing KiCad path for 'osl kicad-edit'")?;
    let output = flag_value(args, "--output").ok_or_else(|| {
        OslError::InvalidInput("missing --output <file.kicad_sch> for 'osl kicad-edit'".to_string())
    })?;
    let input_path = Path::new(input);
    let extension = input_path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    if extension != "kicad_sch" {
        return Err(OslError::InvalidInput(format!(
            "{} is not a supported KiCad edit input (.kicad_sch)",
            input_path.display()
        )));
    }

    let mut schematic = read_kicad_schematic_with_libraries(input_path)?;
    let mut symbol_definitions = schematic.library_symbols.clone();
    for library_path in flag_values(args, "--library") {
        let library = read_kicad_symbol_library(Path::new(&library_path))?;
        symbol_definitions.extend(library.symbols);
    }

    let edits = parse_kicad_edit_ops(args, &symbol_definitions)?;
    if edits.is_empty() {
        return Err(OslError::InvalidInput(
            "kicad-edit requires at least one edit op".to_string(),
        ));
    }

    let mut summaries = Vec::new();
    for edit in edits {
        summaries.push(schematic.apply_edit(edit)?);
    }
    write_kicad_schematic(Path::new(&output), &schematic)?;

    println!("kicad-edit -> {output} ({} edits)", summaries.len());
    for summary in summaries {
        println!("  {} {}", summary.operation, summary.target);
    }
    Ok(0)
}

fn kicad_render_command(args: &[String]) -> OslResult<i32> {
    let input = positional(args, 0, "missing KiCad path for 'osl kicad-render'")?;
    let output = flag_value(args, "--output").ok_or_else(|| {
        OslError::InvalidInput("missing --output <file.svg> for 'osl kicad-render'".to_string())
    })?;
    let input_path = Path::new(input);
    let extension = input_path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    let scene = match extension.as_str() {
        "kicad_sch" => read_kicad_schematic_with_libraries(input_path)?.canvas_scene(),
        "kicad_sym" => {
            let symbol_name = flag_value(args, "--symbol").ok_or_else(|| {
                OslError::InvalidInput(
                    "missing --symbol <name> for rendering a KiCad symbol library".to_string(),
                )
            })?;
            let unit = flag_value(args, "--unit")
                .map(|value| parse_positive_u32(&value, "--unit"))
                .transpose()?;
            let body_style = flag_value(args, "--body-style")
                .map(|value| parse_positive_u32(&value, "--body-style"))
                .transpose()?;
            let library = read_kicad_symbol_library(input_path)?;
            let symbol = library
                .symbol_by_name_or_local_name(&symbol_name)
                .ok_or_else(|| {
                    OslError::InvalidInput(format!(
                        "symbol '{}' was not found in {}",
                        symbol_name,
                        input_path.display()
                    ))
                })?;
            KicadCanvasScene::from_symbol_definition(
                format!("{}:{}", input_path.display(), symbol.local_name()),
                symbol,
                &library.symbols,
                unit,
                body_style,
            )
        }
        _ => {
            return Err(OslError::InvalidInput(format!(
                "{} is not a supported KiCad render input (.kicad_sch, .kicad_sym)",
                input_path.display()
            )));
        }
    };
    let svg = render_kicad_scene_svg(&scene);
    write_text(Path::new(&output), &svg)?;
    println!("kicad-render -> {output}");
    Ok(0)
}

fn copy_import_dependencies(
    report: &ImportReport,
    input_path: &Path,
    project_dir: &Path,
) -> OslResult<Vec<NormalizedDependency>> {
    let base_dir = input_path.parent().unwrap_or_else(|| Path::new("."));
    let mut dependencies = Vec::new();

    for include in &report.includes {
        let include_path = Path::new(&include.path);
        if include_path.is_absolute() {
            continue;
        }
        let Some(file_name) = include_path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let source_path = base_dir.join(include_path);
        if !source_path.is_file() {
            continue;
        }
        let project_path = Path::new("models").join(sanitize_dependency_file_name(file_name));
        let content = read_text(&source_path)?;
        write_text(&project_dir.join(&project_path), &content)?;
        dependencies.push(NormalizedDependency {
            source: include.path.clone(),
            project_path: project_path.display().to_string(),
        });
    }

    Ok(dependencies)
}

fn sanitize_dependency_file_name(file_name: &str) -> String {
    let mut output = String::with_capacity(file_name.len());
    for character in file_name.chars() {
        if character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_') {
            output.push(character);
        } else {
            output.push('_');
        }
    }
    if output.is_empty() {
        "dependency.lib".to_string()
    } else {
        output
    }
}

fn report_command(args: &[String]) -> OslResult<i32> {
    let dir = PathBuf::from(positional(args, 0, "missing directory for 'osl report'")?);
    let output_path = write_report_directory_html(&dir)?;
    println!("report -> {}", output_path.display());
    Ok(0)
}
