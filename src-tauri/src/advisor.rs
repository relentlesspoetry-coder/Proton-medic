//! Evidence-based Proton version advice. Read-only and entirely offline.
//! A recommendation is a proposed test, not proof that a game will launch.
use crate::diagnostics::{DiagnosticsReport, GameDiagnostics};
use crate::scanner::{CompatibilityTool, Game, ScanReport};
use serde::Serialize;
use std::cmp::Ordering;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionAdvice {
    pub app_id: String,
    pub library_path: String,
    pub game_name: String,
    pub status: String,
    pub suggested_tool: Option<String>,
    pub configured_tool: Option<String>,
    pub reason: String,
    pub next_step: String,
    pub confidence: String,
    pub alternatives: Vec<String>,
    pub evidence: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdvisorReport {
    pub games: Vec<VersionAdvice>,
    pub note: String,
}

fn is_experimental(name: &str) -> bool {
    name.to_ascii_lowercase().contains("experimental")
}
fn is_ge(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.contains("ge-proton") || lower.contains("proton-ge")
}
fn is_official_stable(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.starts_with("proton ") && !lower.contains("experimental") && !lower.contains("hotfix")
        && lower[7..].chars().next().is_some_and(|c| c.is_ascii_digit())
}
fn version_parts(name: &str) -> Vec<u32> {
    // Compare only numeric runs rather than lexicographic names (10 > 9).
    name.split(|c: char| !c.is_ascii_digit())
        .filter_map(|n| (!n.is_empty()).then(|| n.parse::<u32>().ok()).flatten())
        .take(4).collect()
}
fn versions_desc(a: &CompatibilityTool, b: &CompatibilityTool) -> Ordering {
    version_parts(&b.name).cmp(&version_parts(&a.name))
        .then_with(|| a.name.cmp(&b.name))
}
pub(crate) fn installed_versions(tools: &[CompatibilityTool]) -> Vec<&CompatibilityTool> {
    // Discovery can include other custom compatibility tools. They must not be
    // presented as a Proton runtime without evidence.
    let mut versions: Vec<&CompatibilityTool> = tools.iter()
        .filter(|t| is_official_stable(&t.name) || is_experimental(&t.name) || is_ge(&t.name))
        .collect();
    versions.sort_by(|a, b| {
        let tier = |t: &CompatibilityTool| {
            if is_official_stable(&t.name) { 0 } else if is_experimental(&t.name) { 1 } else { 2 }
        };
        tier(a).cmp(&tier(b)).then_with(|| versions_desc(a, b))
    });
    versions
}
fn resolve_configured(configured: &str, tools: &[&CompatibilityTool]) -> String {
    // Steam stores manifest IDs, which can differ from directory/display names.
    // Resolve these before name heuristics so an installed custom runtime is
    // not offered as an alternative to itself. internal_id uses bounded VDF IO.
    if let Some(tool) = tools.iter().find(|t| {
        crate::auto_select::internal_id(&t.name, std::path::Path::new(&t.path))
            .is_ok_and(|id| id == configured)
    }) {
        return tool.name.clone();
    }
    let normalized = configured.to_ascii_lowercase().replace('-', "_");
    if let Some(tool) = tools.iter().find(|t| t.name.eq_ignore_ascii_case(configured)) {
        return tool.name.clone();
    }
    if normalized.contains("experimental") {
        if let Some(tool) = tools.iter().find(|t| is_experimental(&t.name)) { return tool.name.clone(); }
    }
    if normalized.starts_with("proton_") {
        let required = version_parts(configured);
        if let Some(tool) = tools.iter().find(|t| is_official_stable(&t.name)
            && version_parts(&t.name).first() == required.first() && !required.is_empty()) {
            return tool.name.clone();
        }
    }
    configured.to_string()
}

pub(crate) fn advise(game: &Game, diagnosis: Option<&GameDiagnostics>, tools: &[&CompatibilityTool]) -> VersionAdvice {
    let configured = game.configured_tool.as_ref().map(|s| resolve_configured(s, tools));
    let evidence: Vec<String> = diagnosis.map(|d| d.findings.iter().map(|f| f.code.clone()).collect())
        .unwrap_or_default();
    let anticheat_blocked = evidence.iter().any(|code| code == "anticheat-blocked");
    let system_problem = evidence.iter()
        .any(|code| code == "vulkan-init" || code == "missing-shared-lib");
    let stable = tools.iter().find(|t| is_official_stable(&t.name)).map(|t| t.name.clone());
    let experimental = tools.iter().find(|t| is_experimental(&t.name)).map(|t| t.name.clone());
    let ge = tools.iter().find(|t| is_ge(&t.name)).map(|t| t.name.clone());
    let mut alternatives: Vec<String> = [stable.clone(), experimental.clone(), ge.clone()]
        .into_iter().flatten().collect();
    alternatives.dedup();

    let mut advice = VersionAdvice {
        app_id: game.app_id.clone(), library_path: game.library_path.clone(), game_name: game.name.clone(),
        status: "need-information".into(), suggested_tool: None, configured_tool: configured.clone(),
        reason: String::new(), next_step: String::new(), confidence: "limited".into(), alternatives,
        evidence,
    };

    if !game.install_path_exists {
        advice.status = "check-install".into();
        advice.reason = "Steam lists this game, but its installation folder was not found.".into();
        advice.next_step = "Reconnect or mount the game library before changing Proton versions.".into();
        return advice;
    }
    if anticheat_blocked {
        advice.reason = "The log reports an unsupported anti-cheat platform. Proton switching will not bypass this restriction.".into();
        advice.next_step = "Check the game's official Linux support. No compatibility change is recommended for this finding.".into();
        return advice;
    }
    if system_problem {
        advice.status = "check-system".into();
        advice.reason = "The log points to a graphics driver or missing Linux library problem. Changing Proton may not resolve it.".into();
        advice.next_step = "Run System Check first, then collect a fresh Proton log.".into();
        return advice;
    }
    if tools.is_empty() {
        advice.status = "need-proton".into();
        advice.reason = "No installed Proton compatibility tool was discovered in accessible Steam libraries.".into();
        advice.next_step = "In Steam, enable Steam Play and install an official Proton version.".into();
        return advice;
    }

    let log_issues = diagnosis.is_some_and(|d| d.findings.iter().any(|f| f.severity == "error" || f.severity == "warning"));
    if !log_issues {
        if let Some(selected) = configured {
            advice.status = "keep-current".into();
            advice.suggested_tool = Some(selected);
            advice.reason = "Steam already has a version selected for this game. The available log does not justify switching it.".into();
            advice.next_step = "If the game fails, launch it once with Proton logging enabled, then scan again.".into();
        } else {
            advice.status = "steam-default".into();
            advice.reason = "No game-specific compatibility override or confirmed launch failure was found. Steam's default is the safest starting point.".into();
            advice.next_step = "Try launching using Steam's default. If it fails, enable Proton logging and scan again.".into();
        }
        return advice;
    }

    // If a launch has produced relevant errors, suggest a different installed
    // candidate instead of claiming the currently-selected version is correct.
    let candidate = [experimental, stable, ge].into_iter().flatten()
        .find(|name| configured.as_deref() != Some(name.as_str()));
    if let Some(candidate) = candidate {
        advice.suggested_tool = Some(candidate);
        advice.status = "try-version".into();
        advice.confidence = "tentative".into();
        advice.reason = "A Proton log contains possible compatibility failures. This is an alternative installed version to test, not a verified fix.".into();
        advice.next_step = "Select this version in the game's Steam Properties → Compatibility, launch once, then check the result.".into();
    } else {
        advice.status = "need-information".into();
        advice.reason = "The log shows possible failures, but no different installed Proton version is available to compare.".into();
        advice.next_step = "Install Proton Experimental through Steam, then scan again. Keep existing save data and prefixes.".into();
    }
    advice
}

pub(crate) fn advise_for_library(scan: &ScanReport, game: &Game, diagnosis: Option<&GameDiagnostics>) -> VersionAdvice {
    match crate::auto_select::tools_for_library(scan, &game.library_path) {
        Ok(tools) => advise(game, diagnosis, &installed_versions(&tools)),
        Err(error) => {
            let mut result = advise(game, diagnosis, &[]);
            result.status = "need-information".into();
            result.suggested_tool = None;
            result.reason = format!("Cannot identify available Proton versions for this Steam library: {error}");
            result.next_step = "Reconnect the library and scan again before changing Proton.".into();
            result
        }
    }
}

pub fn recommend(scan: &ScanReport, report: &DiagnosticsReport) -> AdvisorReport {
    let games = scan.games.iter().map(|game| {
        let diagnosis = report.games.iter().find(|d| d.app_id == game.app_id && d.library_path == game.library_path);
        advise_for_library(scan, game, diagnosis)
    }).collect();
    AdvisorReport {
        games,
        note: "Offline suggestions based on installed tools, Steam configuration and available logs. No version is certified to work until the user tests the game. No settings are changed.".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn game() -> Game {
        Game { app_id: "555".into(), name: "Fixture Game".into(), library_path: "/tmp/Steam".into(),
            install_path: "/tmp/Steam/common/Fixture".into(), install_path_exists: true,
            manifest_path: "/tmp/Steam/appmanifest_555.acf".into(), prefix_path: "/tmp/Steam/compatdata/555/pfx".into(),
            prefix_exists: false, build_id: None, last_played_at: None, configured_tool: None }
    }
    fn tools() -> Vec<CompatibilityTool> {
        ["Proton 9.0", "Proton 10.0", "Proton Experimental", "GE-Proton9-10"].iter()
            .map(|n| CompatibilityTool { name: (*n).into(), path: format!("/test/{n}"), source: "test".into() }).collect()
    }
    fn diag(code: &str) -> GameDiagnostics {
        GameDiagnostics { app_id: "555".into(), library_path: "/tmp/Steam".into(),
            log_path: None, log_size: None, modified_unix_seconds: None, truncated: false,
            status: "review".into(), notes: vec![], findings: vec![crate::diagnostics::Finding {
                code: code.into(), severity: "error".into(), title: "Test".into(),
                description: "Test".into(), recommendation: "Test".into(), evidence: vec!["Test".into()],
            }] }
    }
    #[test] fn versions_are_sorted_numerically() {
        let data = tools();
        assert_eq!(installed_versions(&data)[0].name, "Proton 10.0");
    }
    #[test] fn keeps_user_override_when_no_evidence_of_failure() {
        let mut game = game(); game.configured_tool = Some("proton_9".into());
        let data = tools();
        let result = advise(&game, None, &installed_versions(&data));
        assert_eq!(result.status, "keep-current");
        assert_eq!(result.suggested_tool.as_deref(), Some("Proton 9.0"));
    }
    #[test] fn recommends_other_installed_version_after_exception() {
        let mut game = game(); game.configured_tool = Some("proton_9".into());
        let data = tools(); let diagnostics = diag("wine-exception");
        let result = advise(&game, Some(&diagnostics), &installed_versions(&data));
        assert_eq!(result.status, "try-version");
        assert_eq!(result.suggested_tool.as_deref(), Some("Proton Experimental"));
    }
    #[test] fn custom_manifest_id_does_not_recommend_current_runtime_again() {
        let dir=std::env::temp_dir().join(format!("pm-advisor-id-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let path=dir.join("GE-Proton11-4");
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(path.join("compatibilitytool.vdf"),r#""compatibilitytools" { "compat_tools" { "GE-Proton11-4-x86_64" { "install_path" "." } } }"#).unwrap();
        let mut game=game();
        game.configured_tool=Some("GE-Proton11-4-x86_64".into());
        let mut tools=vec![CompatibilityTool{name:"GE-Proton11-4".into(),path:path.to_string_lossy().into_owned(),source:"Custom Proton".into()}];
        let diagnosis=diag("wine-exception");
        let result=advise(&game,Some(&diagnosis),&installed_versions(&tools));
        assert_eq!(result.configured_tool.as_deref(),Some("GE-Proton11-4"));
        assert_eq!(result.status,"need-information");
        assert!(result.suggested_tool.is_none());
        // A genuinely different installed candidate remains available.
        std::fs::create_dir(dir.join("Proton - Experimental")).unwrap();
        tools.push(CompatibilityTool{name:"Proton - Experimental".into(),path:dir.join("Proton - Experimental").to_string_lossy().into_owned(),source:"Steam library".into()});
        let result=advise(&game,Some(&diagnosis),&installed_versions(&tools));
        assert_eq!(result.status,"try-version");
        assert_eq!(result.suggested_tool.as_deref(),Some("Proton - Experimental"));
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test] fn does_not_propose_version_for_vulkan_driver_failure() {
        let data = tools(); let diagnostics = diag("vulkan-init");
        let result = advise(&game(), Some(&diagnostics), &installed_versions(&data));
        assert_eq!(result.status, "check-system");
        assert!(result.suggested_tool.is_none());
    }
    #[test] fn uses_steam_default_without_launch_error_or_override() {
        let data = tools(); let result = advise(&game(), None, &installed_versions(&data));
        assert_eq!(result.status, "steam-default");
    }
    #[test] fn system_failures_preserve_evidence_without_proton_switch() {
        for code in ["vulkan-init", "missing-shared-lib"] {
            // System diagnosis must take precedence even without installed tools.
            let diagnostics = diag(code);
            let result = advise(&game(), Some(&diagnostics), &[]);
            assert_eq!(result.status, "check-system");
            assert_eq!(result.evidence, vec![code.to_string()]);
            assert!(result.suggested_tool.is_none());
        }
    }
    #[test] fn missing_install_takes_precedence() {
        let data = tools(); let mut game = game(); game.install_path_exists = false;
        assert_eq!(advise(&game, None, &installed_versions(&data)).status, "check-install");
    }
    #[test] fn unsupported_anticheat_does_not_trigger_auto_selection() {
        let data=tools(); let diagnostics=diag("anticheat-blocked");
        let result=advise(&game(),Some(&diagnostics),&installed_versions(&data));
        assert_eq!(result.status,"need-information");
        assert!(result.suggested_tool.is_none());
    }

}
