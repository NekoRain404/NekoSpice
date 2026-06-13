use super::SimulationProfile;

pub fn inject_profile_directives(netlist: &str, profile: &SimulationProfile) -> String {
    let directives = profile.generate_directives();
    if directives.is_empty() {
        return netlist.to_string();
    }

    // The analysis directive from the profile should REPLACE any existing
    // analysis directive in the netlist (the user's UI choices take priority).
    let analysis_keywords = [".tran", ".ac", ".dc", ".op"];
    let mut output = Vec::new();
    let mut inserted_analysis = false;

    for line in netlist.lines() {
        let trimmed = line.trim();
        let lower = trimmed.to_ascii_lowercase();

        // Skip existing analysis directives — we'll replace them with profile values
        let is_existing_analysis = analysis_keywords.iter().any(|kw| {
            lower.starts_with(kw) && (lower.len() == kw.len() || lower.as_bytes()[kw.len()] == b' ')
        });
        if is_existing_analysis && !inserted_analysis {
            // Replace with the profile's analysis directive
            for directive in &directives {
                output.push(directive.clone());
            }
            inserted_analysis = true;
            continue;
        } else if is_existing_analysis {
            // Skip duplicate analysis directives
            continue;
        }

        // Insert remaining profile directives before `.end`
        if trimmed.eq_ignore_ascii_case(".end") && !inserted_analysis {
            // No analysis directive was found — insert all profile directives
            output.push("* --- NekoSpice simulation profile ---".to_string());
            for directive in &directives {
                output.push(directive.clone());
            }
            inserted_analysis = true;
        }

        output.push(line.to_string());
    }

    // If no `.end` found, append at end
    if !inserted_analysis {
        output.push("* --- NekoSpice simulation profile ---".to_string());
        output.extend(directives);
    }

    // Inject vendor model/subcircuit bodies before .end
    if !profile.vendor_model_bodies.is_empty() {
        let mut final_output = Vec::new();
        for line in &output {
            if line.trim().eq_ignore_ascii_case(".end") {
                final_output.push("* --- NekoSpice vendor models ---".to_string());
                for body in &profile.vendor_model_bodies {
                    final_output.push(body.clone());
                }
                final_output.push(String::new());
            }
            final_output.push(line.clone());
        }
        if final_output.len() > output.len() {
            return final_output.join("\n");
        }
    }

    output.join("\n")
}

/// Validate that a netlist is runnable by ngspice/Xyce.
///
/// Checks for:
/// - Non-empty netlist
/// - Presence of `.end` directive
/// - At least one analysis directive (.tran, .ac, .dc, .op)
///
/// Returns a list of warning/error messages. Empty vec means valid.
pub fn validate_netlist_for_simulation(netlist: &str) -> Vec<String> {
    let mut issues = Vec::new();
    let trimmed = netlist.trim();

    if trimmed.is_empty() {
        issues.push("Netlist is empty".to_string());
        return issues;
    }

    let has_end = trimmed
        .lines()
        .any(|line| line.trim().eq_ignore_ascii_case(".end"));
    if !has_end {
        issues.push("Missing .end directive".to_string());
    }

    let has_analysis = trimmed.lines().any(|line| {
        let lower = line.trim().to_ascii_lowercase();
        lower.starts_with(".tran ")
            || lower.starts_with(".ac ")
            || lower.starts_with(".dc ")
            || lower == ".op"
    });
    if !has_analysis {
        issues.push("No analysis directive found (.tran, .ac, .dc, .op)".to_string());
    }

    issues
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn inject_directives_before_end() {
        let netlist = "* RC filter\nR1 in out 1k\nC1 out 0 100n\n.end\n";
        let mut profile = SimulationProfile::default();
        profile.temperature = "125".to_string();
        let result = inject_profile_directives(netlist, &profile);
        assert!(result.contains(".temp 125"));
        assert!(result.contains(".end"), "netlist should end with .end");
        assert!(
            result.trim_end().ends_with(".end"),
            "netlist should end with .end directive"
        );
        let temp_pos = result.find(".temp 125").unwrap();
        let end_pos = result.find(".end").unwrap();
        assert!(temp_pos < end_pos);
    }

    #[test]
    fn validate_empty_netlist() {
        let issues = validate_netlist_for_simulation("");
        assert!(issues.iter().any(|i| i.contains("empty")));
    }

    #[test]
    fn validate_missing_end() {
        let issues = validate_netlist_for_simulation("* RC\n.tran 1u 1m\nR1 in out 1k\n");
        assert!(issues.iter().any(|i| i.contains(".end")));
    }

    #[test]
    fn validate_missing_analysis() {
        let issues = validate_netlist_for_simulation("* RC\nR1 in out 1k\n.end\n");
        assert!(issues.iter().any(|i| i.contains("analysis")));
    }

    #[test]
    fn validate_valid_netlist() {
        let issues = validate_netlist_for_simulation("* RC\n.tran 1u 1m\nR1 in out 1k\n.end\n");
        assert!(issues.is_empty());
    }
}
