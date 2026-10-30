//! Read-only scanner for Steam libraries, app manifests, and compatibility tools.
//! Never launches Steam or writes to scanned files.
use serde::Serialize;
use std::collections::{BTreeMap, HashSet};
use std::env;
use std::fs;
use std::path::{Component, Path, PathBuf};

const MAX_VDF_BYTES: u64 = 8 * 1024 * 1024;
const MAX_VDF_DEPTH: usize = 24;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanReport {
    pub installations: Vec<SteamInstallation>,
    pub libraries: Vec<SteamLibrary>,
    pub games: Vec<Game>,
    pub compatibility_tools: Vec<CompatibilityTool>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SteamInstallation {
    pub kind: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SteamLibrary {
    pub path: String,
    pub steam_apps_path: String,
    pub install_kind: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Game {
    pub app_id: String,
    pub name: String,
    pub library_path: String,
    pub install_path: String,
    pub install_path_exists: bool,
    pub manifest_path: String,
    pub prefix_path: String,
    pub prefix_exists: bool,
    pub build_id: Option<String>,
    pub configured_tool: Option<String>,
    pub last_played_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompatibilityTool {
    pub name: String,
    pub path: String,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Value {
    Text(String),
    Object(Vec<(String, Value)>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token { Text(String), Open, Close }

fn tokenize(data: &str) -> Result<Vec<Token>, String> {
    let input = data.trim_start_matches('\u{feff}').as_bytes();
    let mut output = Vec::new();
    let mut i = 0;
    while i < input.len() {
        match input[i] {
            b' ' | b'\n' | b'\r' | b'\t' => i += 1,
            b'/' if i + 1 < input.len() && input[i + 1] == b'/' => {
                i += 2;
                while i < input.len() && input[i] != b'\n' { i += 1; }
            }
            b'/' if i + 1 < input.len() && input[i + 1] == b'*' => {
                i += 2;
                let mut closed = false;
                while i + 1 < input.len() {
                    if input[i] == b'*' && input[i + 1] == b'/' {
                        i += 2;
                        closed = true;
                        break;
                    }
                    i += 1;
                }
                if !closed { return Err("unterminated block comment".into()); }
            }
            b'{' => { output.push(Token::Open); i += 1; }
            b'}' => { output.push(Token::Close); i += 1; }
            b'"' => {
                i += 1;
                let mut bytes = Vec::new();
                let mut closed = false;
                while i < input.len() {
                    if input[i] == b'"' { i += 1; closed = true; break; }
                    if input[i] == b'\\' && i + 1 < input.len() {
                        match input[i + 1] {
                            b'"' | b'\\' => { bytes.push(input[i + 1]); i += 2; continue; }
                            b'n' => { bytes.push(b'\n'); i += 2; continue; }
                            b't' => { bytes.push(b'\t'); i += 2; continue; }
                            _ => {} // Preserve unknown escapes, including Windows-style paths.
                        }
                    }
                    bytes.push(input[i]); i += 1;
                }
                if !closed { return Err("unterminated quoted string".into()); }
                output.push(Token::Text(String::from_utf8_lossy(&bytes).into_owned()));
            }
            _ => {
                let start = i;
                while i < input.len() && !input[i].is_ascii_whitespace() && input[i] != b'{' && input[i] != b'}' {
                    i += 1;
                }
                output.push(Token::Text(String::from_utf8_lossy(&input[start..i]).into_owned()));
            }
        }
        if output.len() > 200_000 { return Err("too many VDF tokens".into()); }
    }
    Ok(output)
}

fn parse_pairs(tokens: &[Token], pos: &mut usize, depth: usize, nested: bool) -> Result<Vec<(String, Value)>, String> {
    if depth > MAX_VDF_DEPTH { return Err("VDF nesting limit reached".into()); }
    let mut pairs = Vec::new();
    while *pos < tokens.len() {
        let key = match &tokens[*pos] {
            Token::Close => {
                if !nested { return Err("unexpected closing brace".into()); }
                *pos += 1;
                return Ok(pairs);
            }
            Token::Open => return Err("unexpected opening brace".into()),
            Token::Text(key) => key.clone(),
        };
        *pos += 1;
        if *pos >= tokens.len() { return Err(format!("missing value for key {key}")); }
        let value = match &tokens[*pos] {
            Token::Open => {
                *pos += 1;
                Value::Object(parse_pairs(tokens, pos, depth + 1, true)?)
            }
            Token::Close => return Err(format!("missing value for key {key}")),
            Token::Text(text) => {
                let value = Value::Text(text.clone());
                *pos += 1;
                value
            }
        };
        pairs.push((key, value));
    }
    if nested { Err("missing closing brace".into()) } else { Ok(pairs) }
}

pub(crate) fn parse_vdf(data: &str) -> Result<Vec<(String, Value)>, String> {
    let tokens = tokenize(data)?;
    let mut position = 0;
    parse_pairs(&tokens, &mut position, 0, false)
}

pub(crate) fn obj_get<'a>(entries: &'a [(String, Value)], key: &str) -> Option<&'a [(String, Value)]> {
    entries.iter().find_map(|(k, v)| match v {
        Value::Object(items) if k.eq_ignore_ascii_case(key) => Some(items.as_slice()),
        _ => None,
    })
}

pub(crate) fn text_get<'a>(entries: &'a [(String, Value)], key: &str) -> Option<&'a str> {
    entries.iter().find_map(|(k, v)| match v {
        Value::Text(s) if k.eq_ignore_ascii_case(key) => Some(s.as_str()),
        _ => None,
    })
}

pub(crate) fn load_vdf(path: &Path) -> Result<Vec<(String, Value)>, String> {
    let size = fs::metadata(path).map_err(|e| e.to_string())?.len();
    if size > MAX_VDF_BYTES { return Err("manifest exceeds size limit".into()); }
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    parse_vdf(&String::from_utf8_lossy(&bytes))
}

fn clean_path(path: &Path) -> PathBuf { fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf()) }
fn display_path(path: &Path) -> String { path.to_string_lossy().into_owned() }
fn is_digits(s: &str) -> bool { !s.is_empty() && s.len() <= 20 && s.bytes().all(|b| b.is_ascii_digit()) }
fn safe_install_dir(s: &str) -> bool {
    let mut parts = Path::new(s).components();
    matches!((parts.next(), parts.next()), (Some(Component::Normal(_)), None))
}

fn installation_candidates(home: &Path) -> Vec<(&'static str, PathBuf)> {
    vec![
        ("Native", home.join(".local/share/Steam")),
        ("Native", home.join(".steam/steam")),
        ("Native", home.join(".steam/root")),
        ("Native", home.join(".steam/debian-installation")),
        ("Flatpak", home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam")),
        ("Flatpak", home.join(".var/app/com.valvesoftware.Steam/data/Steam")),
    ]
}

pub(crate) fn libraries_from_vdf(steam_root: &Path, warnings: &mut Vec<String>) -> Vec<PathBuf> {
    let mut libs = vec![steam_root.to_path_buf()];
    let vdf_path = steam_root.join("steamapps/libraryfolders.vdf");
    if !vdf_path.is_file() { return libs; }
    match load_vdf(&vdf_path) {
        Ok(entries) => {
            if let Some(folders) = obj_get(&entries, "libraryfolders") {
                for (key, value) in folders {
                    if !is_digits(key) { continue; }
                    let candidate = match value {
                        Value::Text(path) => Some(path.as_str()),
                        Value::Object(items) => text_get(items, "path"),
                    };
                    if let Some(raw) = candidate {
                        let path = PathBuf::from(raw);
                        if path.is_absolute() { libs.push(path); }
                        else { warnings.push(format!("Ignored non-absolute Steam library path: {raw}")); }
                    }
                }
            } else {
                warnings.push(format!("libraryfolders.vdf missing libraryfolders object: {}", display_path(&vdf_path)));
            }
        }
        Err(error) => warnings.push(format!("Could not parse {}: {error}", display_path(&vdf_path))),
    }
    libs
}

// Steam stores per-app compatibility overrides in config/config.vdf. The mapping
// stores Steam's internal tool IDs, which need not match the installation folder.
// Read only; do not edit Steam config while Steam is running.
fn extract_tool_mapping(entries: &[(String, Value)], result: &mut BTreeMap<String, String>, depth: usize) {
    if depth > MAX_VDF_DEPTH { return; }
    for (key, value) in entries {
        let Value::Object(children) = value else { continue; };
        if key.eq_ignore_ascii_case("CompatToolMapping") {
            for (app_id, mapping) in children {
                if !is_digits(app_id) || app_id == "0" { continue; }
                if let Value::Object(props) = mapping {
                    if let Some(name) = text_get(props, "name") {
                        if !name.trim().is_empty() {
                            result.insert(app_id.clone(), name.to_string());
                        }
                    }
                }
            }
        } else {
            extract_tool_mapping(children, result, depth + 1);
        }
    }
}

fn read_tool_mapping(root: &Path, warnings: &mut Vec<String>) -> BTreeMap<String, String> {
    let config = root.join("config/config.vdf");
    if !config.is_file() { return BTreeMap::new(); }
    match load_vdf(&config) {
        Ok(entries) => {
            let mut mapping = BTreeMap::new();
            extract_tool_mapping(&entries, &mut mapping, 0);
            mapping
        }
        Err(error) => {
            warnings.push(format!("Could not read Steam compatibility selections from {}: {error}", display_path(&config)));
            BTreeMap::new()
        }
    }
}

fn utility_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower == "proton" || lower.starts_with("proton ") || lower.starts_with("proton-") ||
        lower.starts_with("ge-proton") || lower.starts_with("steam linux runtime") ||
        lower.starts_with("steamworks common redistributables") || lower.starts_with("steam runtime")
}

fn discover_compatibility_tools(folder: &Path, source: &str, seen: &mut HashSet<PathBuf>, results: &mut Vec<CompatibilityTool>) {
    let Ok(dirs) = fs::read_dir(folder) else { return; };
    for dir in dirs.flatten() {
        let path = dir.path();
        if !path.is_dir() { continue; }
        let name = dir.file_name().to_string_lossy().into_owned();
        // Custom tools declare a compatibilitytool.vdf. Include known Proton-style tools as well.
        if !path.join("compatibilitytool.vdf").is_file() && !utility_name(&name) { continue; }
        let canonical = clean_path(&path);
        if seen.insert(canonical.clone()) {
            results.push(CompatibilityTool { name, path: display_path(&canonical), source: source.into() });
        }
    }
}

pub fn scan() -> ScanReport {
    let home = env::var_os("HOME").map(PathBuf::from);
    match home {
        Some(path) => scan_home(&path),
        None => ScanReport {
            installations: vec![], libraries: vec![], games: vec![], compatibility_tools: vec![],
            warnings: vec!["HOME environment variable is not set; cannot locate Steam files.".into()],
        },
    }
}

pub(crate) fn scan_home(home: &Path) -> ScanReport {
    let mut report = ScanReport {
        installations: vec![], libraries: vec![], games: vec![], compatibility_tools: vec![], warnings: vec![],
    };
    let mut seen_roots = HashSet::new();
    let mut seen_libraries = HashSet::new();
    let mut seen_tools = HashSet::new();
    let mut game_keys = HashSet::<(String, PathBuf)>::new();
    let mut libs = Vec::<(PathBuf, String)>::new();
    let mut overrides = BTreeMap::<PathBuf, BTreeMap<String, String>>::new();
    let mut owners = BTreeMap::<PathBuf, usize>::new();

    for (kind, candidate) in installation_candidates(home) {
        if !candidate.join("steamapps").is_dir() { continue; }
        let root = clean_path(&candidate);
        if !seen_roots.insert(root.clone()) { continue; }
        report.installations.push(SteamInstallation { kind: kind.into(), path: display_path(&root) });
        let mapping = read_tool_mapping(&root, &mut report.warnings);
        let mut owned_libraries = HashSet::new();
        for lib in libraries_from_vdf(&root, &mut report.warnings) {
            if !lib.join("steamapps").is_dir() {
                report.warnings.push(format!("Steam library unavailable or inaccessible: {}", display_path(&lib)));
                continue;
            }
            let canonical = clean_path(&lib);
            if !owned_libraries.insert(canonical.clone()) { continue; }
            *owners.entry(canonical.clone()).or_default() += 1;
            overrides.insert(canonical.clone(), mapping.clone());
            if seen_libraries.insert(canonical.clone()) {
                libs.push((canonical, kind.into()));
            }
        }
        discover_compatibility_tools(&root.join("compatibilitytools.d"), "Custom Proton", &mut seen_tools, &mut report.compatibility_tools);
    }

    // In case Steam roots are symlinks to the same directory, libraries are scanned once.
    for (lib, kind) in libs {
        let steamapps = lib.join("steamapps");
        report.libraries.push(SteamLibrary {
            path: display_path(&lib), steam_apps_path: display_path(&steamapps), install_kind: if owners.get(&lib).copied().unwrap_or(0) == 1 { kind } else { "Ambiguous".into() },
        });
        let Ok(entries) = fs::read_dir(&steamapps) else {
            report.warnings.push(format!("Cannot read Steam library: {}", display_path(&steamapps)));
            continue;
        };
        discover_compatibility_tools(&steamapps.join("common"), "Steam library", &mut seen_tools, &mut report.compatibility_tools);
        for entry in entries.flatten() {
            let file_name = entry.file_name().to_string_lossy().into_owned();
            let Some(app_id) = file_name.strip_prefix("appmanifest_").and_then(|s| s.strip_suffix(".acf")) else { continue; };
            if !is_digits(app_id) { continue; }
            let manifest_path = entry.path();
            if !manifest_path.is_file() { continue; }
            let parsed = match load_vdf(&manifest_path) {
                Ok(v) => v,
                Err(e) => {
                    report.warnings.push(format!("Could not parse {}: {e}", display_path(&manifest_path)));
                    continue;
                }
            };
            let Some(state) = obj_get(&parsed, "AppState") else {
                report.warnings.push(format!("Manifest lacks AppState: {}", display_path(&manifest_path)));
                continue;
            };
            let Some(manifest_id) = text_get(state, "appid") else {
                report.warnings.push(format!("Manifest lacks appid: {}", display_path(&manifest_path)));
                continue;
            };
            if manifest_id != app_id {
                report.warnings.push(format!("App ID mismatch in {}", display_path(&manifest_path)));
                continue;
            }
            let name = text_get(state, "name").unwrap_or("Unknown Steam application").trim();
            if utility_name(name) { continue; }
            let Some(dir) = text_get(state, "installdir") else {
                report.warnings.push(format!("Manifest lacks installdir: {}", display_path(&manifest_path)));
                continue;
            };
            if !safe_install_dir(dir) {
                report.warnings.push(format!("Unsafe installdir ignored in {}", display_path(&manifest_path)));
                continue;
            }
            if !game_keys.insert((app_id.into(), lib.clone())) { continue; }
            let install_path = steamapps.join("common").join(dir);
            let prefix_path = steamapps.join("compatdata").join(app_id).join("pfx");
            report.games.push(Game {
                app_id: app_id.into(), name: name.into(), library_path: display_path(&lib),
                install_path_exists: install_path.is_dir(), install_path: display_path(&install_path),
                manifest_path: display_path(&manifest_path),
                prefix_exists: prefix_path.is_dir(), prefix_path: display_path(&prefix_path),
                build_id: text_get(state, "buildid").map(str::to_string),
                configured_tool: if owners.get(&lib).copied().unwrap_or(0) == 1 { overrides.get(&lib).and_then(|m|m.get(app_id)).cloned() } else { None },
                last_played_at: played_timestamp(text_get(state, "LastPlayed")),
            });
        }
    }
    report.games.sort_by(|a, b| a.name.to_ascii_lowercase().cmp(&b.name.to_ascii_lowercase()).then(a.app_id.cmp(&b.app_id)));
    report.libraries.sort_by(|a, b| a.path.cmp(&b.path));
    report.compatibility_tools.sort_by(|a, b| a.name.to_ascii_lowercase().cmp(&b.name.to_ascii_lowercase()));
    report.warnings.sort();
    if report.installations.is_empty() {
        report.warnings.push("No supported Steam installation found in this user's home directory.".into());
    }
    report
}

fn played_timestamp(value: Option<&str>) -> Option<u64> {
    let seconds=value?.parse::<u64>().ok()?;
    let now=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).ok()?.as_secs();
    (seconds>=946684800 && seconds<=now).then_some(seconds.checked_mul(1000)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

    struct TestHome(PathBuf);
    impl TestHome {
        fn new() -> Self {
            let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
            let path = env::temp_dir().join(format!("pm-test-{}-{}", std::process::id(), id));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for TestHome { fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); } }

    #[test] fn native_and_flatpak_mappings_keep_library_ownership() {
        let home=TestHome::new();
        for (relative,tool) in [(".local/share/Steam","native-proton"),(".var/app/com.valvesoftware.Steam/data/Steam","flatpak-proton")] {
            let root=home.0.join(relative);
            fs::create_dir_all(root.join("steamapps/common/Game")).unwrap();
            fs::create_dir_all(root.join("config")).unwrap();
            fs::write(root.join("steamapps/appmanifest_123.acf"),r#""AppState" { "appid" "123" "name" "Game" "installdir" "Game" }"#).unwrap();
            fs::write(root.join("config/config.vdf"),format!(r#""CompatToolMapping" {{ "123" {{ "name" "{tool}" }} }}"#)).unwrap();
            fs::write(root.join("steamapps/libraryfolders.vdf"),format!(r#""libraryfolders" {{ "0" {{ "path" "{}" }} }}"#,root.display())).unwrap();
        }
        let scan=scan_home(&home.0);
        assert_eq!(scan.games.len(),2);
        for game in &scan.games {
            let flatpak=game.library_path.contains(".var/app");
            assert_eq!(game.configured_tool.as_deref(),Some(if flatpak {"flatpak-proton"} else {"native-proton"}));
            let paths=crate::diagnostics::owned_log_candidates(&home.0,"123",&game.library_path,&scan);
            assert_eq!(paths.is_empty(),!flatpak);
            assert!(paths.iter().all(|p|p.starts_with(home.0.join(".var/app"))));
            let mut single=scan.clone();
            single.installations.retain(|i| (i.kind=="Flatpak")==flatpak);
            let owned=crate::diagnostics::owned_log_candidates(&home.0,"123",&game.library_path,&single);
            assert!(owned.contains(&home.0.join("steam-123.log")));
            if !flatpak { assert_eq!(owned.len(),1); }
        }
        let mut ambiguous=scan.clone();
        ambiguous.libraries[0].install_kind="Ambiguous".into();
        assert!(crate::diagnostics::owned_log_candidates(&home.0,"123",&ambiguous.libraries[0].path,&ambiguous).is_empty());
    }

    #[test] fn last_played_dates_ignore_missing_zero_invalid_and_future_values(){
        assert_eq!(played_timestamp(Some("1700000000")),Some(1700000000000));
        for value in [None,Some("0"),Some("86400"),Some("-1"),Some("bad"),Some("18446744073709551615")]{assert_eq!(played_timestamp(value),None);}
    }
    #[test]
    fn parse_vdf_quoted_escaped_comments_and_nested_values() {
        let vdf = "// comment\n\"AppState\" { \"name\" \"It's \\\"Working\\\"\" /* x */ \"buildid\" \"17\" }";
        let parsed = parse_vdf(vdf).unwrap();
        let state = obj_get(&parsed, "AppState").unwrap();
        assert_eq!(text_get(state, "name"), Some("It's \"Working\""));
        assert_eq!(text_get(state, "buildid"), Some("17"));
    }

    #[test]
    fn quoted_brace_keys_and_values_remain_text() {
        let parsed = parse_vdf(r#""{" "}" "}" "{" "nested" { "{" "}" "after" "kept" }"#).unwrap();
        assert_eq!(text_get(&parsed, "{"), Some("}"));
        assert_eq!(text_get(&parsed, "}"), Some("{"));
        let nested = obj_get(&parsed, "nested").unwrap();
        assert_eq!(text_get(nested, "{"), Some("}"));
        assert_eq!(text_get(nested, "after"), Some("kept"));
    }

    #[test]
    fn login_users_preserves_brace_persona_names_and_account_metadata() {
        for name in ["{", "}"] {
            let vdf = format!(r#""users" {{ "76561197960265729" {{ "PersonaName" "{name}" "MostRecent" "1" }} }}"#);
            let parsed = parse_vdf(&vdf).unwrap();
            let user = obj_get(obj_get(&parsed, "users").unwrap(), "76561197960265729").unwrap();
            assert_eq!(text_get(user, "PersonaName"), Some(name));
            assert_eq!(text_get(user, "MostRecent"), Some("1"));
        }
    }

    #[test]
    fn quoted_braces_do_not_balance_structural_braces() {
        for vdf in [r#""root" { "name" "}""#, r#""root" { "name" }"#, r#""root" "{" }"#, "{ key value }"] {
            assert!(parse_vdf(vdf).is_err(), "accepted malformed VDF: {vdf}");
        }
    }

    #[test]
    fn malformed_vdf_is_rejected() {
        assert!(parse_vdf("\"AppState\" { \"name\" \"Game\"").is_err());
        assert!(parse_vdf("\"AppState\" { \"name\"").is_err());
        assert!(parse_vdf("}").is_err());
    }

    #[test]
    fn rejects_path_traversal() {
        assert!(safe_install_dir("My Game"));
        assert!(!safe_install_dir("../Games"));
        assert!(!safe_install_dir("/tmp"));
        assert!(!safe_install_dir("folder/subfolder"));
    }

    #[test]
    fn scans_multiple_libraries_prefixes_and_tools() {
        let home = TestHome::new();
        let root = home.0.join(".local/share/Steam");
        let extra = home.0.join("Extra Steam Library");
        fs::create_dir_all(root.join("steamapps/common")).unwrap();
        fs::create_dir_all(root.join("compatibilitytools.d/GE-Proton9-1")).unwrap();
        fs::create_dir_all(extra.join("steamapps/common/Second Game")).unwrap();
        fs::create_dir_all(extra.join("steamapps/compatdata/222/pfx")).unwrap();
        fs::write(root.join("steamapps/libraryfolders.vdf"), format!(
            "\"libraryfolders\" {{ \"0\" {{ \"path\" \"{}\" }} \"1\" {{ \"path\" \"{}\" }} }}",
            root.display(), extra.display()
        )).unwrap();
        fs::write(root.join("steamapps/appmanifest_111.acf"),
            "\"AppState\" { \"appid\" \"111\" \"name\" \"First Game\" \"installdir\" \"First Game\" \"LastPlayed\" \"1700000000\" }").unwrap();
        fs::write(extra.join("steamapps/appmanifest_222.acf"),
            "\"AppState\" { \"appid\" \"222\" \"name\" \"Second Game\" \"installdir\" \"Second Game\" }").unwrap();
        let result = scan_home(&home.0);
        assert_eq!(result.installations.len(), 1);
        assert_eq!(result.libraries.len(), 2);
        assert_eq!(result.games.len(), 2);
        assert_eq!(result.games.iter().find(|g|g.app_id=="111").unwrap().last_played_at,Some(1700000000000));
        assert_eq!(result.games.iter().find(|g|g.app_id=="222").unwrap().last_played_at,None);
        assert_eq!(result.compatibility_tools.len(), 1);
        assert!(result.games.iter().any(|g| g.app_id == "222" && g.prefix_exists && g.install_path_exists));
        assert!(result.games.iter().any(|g| g.app_id == "111" && !g.prefix_exists && !g.install_path_exists));
    }

    #[test]
    fn finds_per_game_override_without_confusing_global_default() {
        let home = TestHome::new();
        let root = home.0.join(".local/share/Steam");
        fs::create_dir_all(root.join("config")).unwrap();
        fs::create_dir_all(root.join("steamapps/common/Test Game")).unwrap();
        fs::write(root.join("steamapps/appmanifest_111.acf"),
            r#""AppState" { "appid" "111" "name" "Test Game" "installdir" "Test Game" }"#).unwrap();
        fs::write(root.join("config/config.vdf"), r#""InstallConfigStore" {
          "Software" { "Valve" { "Steam" { "CompatToolMapping" {
            "0" { "name" "proton_default" }
            "111" { "name" "proton_experimental" "Priority" "250" }
          } } } } }"#).unwrap();
        let report = scan_home(&home.0);
        assert_eq!(report.games.len(), 1);
        assert_eq!(report.games[0].configured_tool.as_deref(), Some("proton_experimental"));
    }

    #[test]
    fn finds_flatpak_steam_and_ignores_invalid_manifest() {
        let home = TestHome::new();
        let root = home.0.join(".var/app/com.valvesoftware.Steam/.local/share/Steam");
        fs::create_dir_all(root.join("steamapps")).unwrap();
        fs::write(root.join("steamapps/appmanifest_123.acf"),
            "\"AppState\" { \"appid\" \"999\" \"name\" \"Fake\" \"installdir\" \"Fake\" }").unwrap();
        let result = scan_home(&home.0);
        assert_eq!(result.installations[0].kind, "Flatpak");
        assert!(result.games.is_empty());
        assert!(result.warnings.iter().any(|w| w.contains("mismatch")));
    }
}
