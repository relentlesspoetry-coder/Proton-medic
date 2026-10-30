//! Local-only, read-only Proton diagnostics.
//! No shell, files written, game execution, network access, or prefix modifications.
use crate::scanner::ScanReport;
use serde::Serialize;
use std::env;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const MAX_LOG_BYTES: u64 = 24 * 1024 * 1024;
const LOG_TAIL_BYTES: u64 = 2 * 1024 * 1024;
const MAX_EVIDENCE: usize = 3;
const MAX_MATCHES: usize = 20;
const MAX_OUTPUT: usize = 128 * 1024;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    pub code: String,
    pub severity: String,
    pub title: String,
    pub description: String,
    pub recommendation: String,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameDiagnostics {
    pub app_id: String,
    pub library_path: String,
    pub log_path: Option<String>,
    pub log_size: Option<u64>,
    pub modified_unix_seconds: Option<u64>,
    pub truncated: bool,
    pub status: String, // 'no-log', 'review', 'no-signatures', 'unreadable'
    pub findings: Vec<Finding>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsReport {
    pub games: Vec<GameDiagnostics>,
    pub system: SystemInfo,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfo {
    pub os: String,
    pub kernel: String,
    pub arch: String,
    pub gpu_devices: Vec<String>,
    pub vulkaninfo_path: Option<String>,
    pub icd_manifests: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VulkanProbe {
    pub status: String, // available, unavailable, timeout, failed
    pub exit_code: Option<i32>,
    pub output: String,
    pub detail: String,
}

fn compact_evidence(s: &str) -> String {
    let normalized = s.chars().map(|c| if c.is_control() { ' ' } else { c }).collect::<String>();
    let trimmed = normalized.trim();
    // Evidence is always a short excerpt; do not export entire log lines.
    trimmed.chars().take(260).collect()
}

#[derive(Clone, Copy)]
struct Signature {
    code: &'static str,
    severity: &'static str,
    title: &'static str,
    description: &'static str,
    recommendation: &'static str,
    patterns: &'static [&'static str],
}

const SIGNATURES: &[Signature] = &[
    Signature {
        code: "anticheat-blocked", severity: "error", title: "Unsupported anti-cheat platform reported",
        description: "The log explicitly reports that an anti-cheat component does not support this platform.",
        recommendation: "Check the developer's supported platforms. Proton Medic does not bypass anti-cheat.",
        patterns: &["easy anti-cheat is not supported on this platform", "battleye: unsupported platform"],
    },
    Signature {
        code: "vulkan-init", severity: "error", title: "Vulkan initialization failure detected",
        description: "The log contains a Vulkan initialization or device-creation error. The root cause is not yet proven.",
        recommendation: "Run the Vulkan system probe, check the selected GPU and the 32-bit/64-bit Vulkan driver packages, then retry the game.",
        patterns: &["failed to create vulkan instance", "failed to initialize vulkan", "vk_error_incompatible_driver", "vk_error_initialization_failed", "no vulkan devices found", "vulkan: failed to"],
    },
    Signature {
        code: "vulkan-device-lost", severity: "error", title: "Vulkan device-loss message",
        description: "A device-lost error may indicate a GPU reset, driver defect, or graphics workload issue.",
        recommendation: "Check GPU driver updates and system graphics logs; test a different Proton version without deleting the prefix.",
        patterns: &["vk_error_device_lost", "dxgi_error_device_removed", "dxgi_error_device_hung"],
    },
    Signature {
        code: "missing-shared-lib", severity: "error", title: "Linux shared-library load failure",
        description: "A process reported a missing shared object or linker load failure. Review the exact library in the evidence.",
        recommendation: "Identify the missing library and whether the game uses Steam Runtime, native Steam, or Flatpak before installing anything.",
        patterns: &["error while loading shared libraries", "cannot open shared object file", "failed to load libvulkan.so", "libvulkan.so.1: cannot open"],
    },
    Signature {
        code: "wine-exception", severity: "warning", title: "Windows exception or page fault",
        description: "The log includes an unhandled Wine exception; this alone does not identify its underlying cause.",
        recommendation: "Inspect the lines surrounding the exception, collect a fresh log, and compare with another Proton version.",
        patterns: &["unhandled exception", "unhandled page fault", "exception code: c0000005", "status_access_violation"],
    },
    Signature {
        code: "memory", severity: "warning", title: "Memory allocation failure",
        description: "The log references an allocation failure or memory exhaustion; it does not identify whether VRAM or RAM is responsible.",
        recommendation: "Check available system memory, VRAM usage, and graphics settings while reproducing the failure.",
        patterns: &["std::bad_alloc", "vk_error_out_of_device_memory", "vk_error_out_of_host_memory", "out of memory error", "failed to allocate memory"],
    },
    Signature {
        code: "wine-module", severity: "warning", title: "Wine module or DLL could not be loaded",
        description: "A required module may be missing, blocked, or incompatible. Some module messages are harmless; context matters.",
        recommendation: "Inspect the named module and nearby log context before changing any Wine prefix components.",
        patterns: &["import_dll library", "module not found", "status_dll_not_found", "c0000135"],
    },
    Signature {
        code: "anticheat", severity: "info", title: "Anti-cheat reference found",
        description: "Anti-cheat components are mentioned in the log. This is not evidence that anti-cheat caused a failure.",
        recommendation: "Check the game's official Linux/Proton anti-cheat support before attempting any compatibility changes.",
        patterns: &["easyanticheat", "easy anti-cheat", "battleye", "battl eye"],
    },
];

fn parse_log(text: &str) -> Vec<Finding> {
    let mut matches: Vec<Vec<String>> = vec![Vec::new(); SIGNATURES.len()];
    for line in text.lines().rev().take(30_000) {
        let lower = line.to_ascii_lowercase();
        for (i, signature) in SIGNATURES.iter().enumerate() {
            if matches[i].len() >= MAX_EVIDENCE { continue; }
            if signature.patterns.iter().any(|pattern| lower.contains(pattern)) {
                matches[i].push(compact_evidence(line));
            }
        }
        if matches.iter().map(Vec::len).sum::<usize>() >= MAX_MATCHES { break; }
    }
    SIGNATURES.iter().zip(matches).filter_map(|(signature, mut evidence)| {
        if evidence.is_empty() { return None; }
        evidence.reverse();
        Some(Finding {
            code: signature.code.into(), severity: signature.severity.into(), title: signature.title.into(),
            description: signature.description.into(), recommendation: signature.recommendation.into(), evidence,
        })
    }).collect()
}

fn read_tail(path: &Path) -> Result<(String, u64, bool), String> {
    let mut file = File::open(path).map_err(|e| format!("Cannot open log: {e}"))?;
    let len = file.metadata().map_err(|e| e.to_string())?.len();
    if len > MAX_LOG_BYTES { return Err(format!("Log exceeds the {} MiB safety limit.", MAX_LOG_BYTES / 1024 / 1024)); }
    let offset = len.saturating_sub(LOG_TAIL_BYTES);
    file.seek(SeekFrom::Start(offset)).map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    file.take(LOG_TAIL_BYTES).read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    if offset > 0 {
        if let Some(first_line_end) = bytes.iter().position(|b| *b == b'\n') { bytes.drain(..=first_line_end); }
    }
    Ok((String::from_utf8_lossy(&bytes).into_owned(), len, offset > 0))
}

pub(crate) fn log_candidates(home: &Path, app_id: &str) -> Vec<PathBuf> {
    // app_id is obtained from scanner, which validates digits in the appmanifest filename.
    if app_id.is_empty() || !app_id.bytes().all(|byte| byte.is_ascii_digit()) { return vec![]; }
    let filename = format!("steam-{app_id}.log");
    vec![
        home.join(&filename),
        home.join(".var/app/com.valvesoftware.Steam").join(&filename),
        home.join(".var/app/com.valvesoftware.Steam/data").join(filename),
    ]
}

fn empty_game(app_id: &str, library_path: &str) -> GameDiagnostics {
    GameDiagnostics {
        app_id: app_id.into(), library_path: library_path.into(), log_path: None,
        log_size: None, modified_unix_seconds: None, truncated: false,
        status: "no-log".into(), findings: vec![], notes: vec![],
    }
}

// Analyze the exact log that changed during this launch attempt. Selecting
// the newest log across every Steam location here can accidentally pick an
// unrelated, future-dated, or stale log and suppress this launch's evidence.
pub(crate) fn analyze_exact_log(path: &Path, app_id: &str, library_path: &str) -> GameDiagnostics {
    let mut result = empty_game(app_id, library_path);
    // In particular, never follow a symlink masquerading as a Proton log.
    let meta = match fs::symlink_metadata(path) {
        Ok(meta) if meta.is_file() && !meta.file_type().is_symlink() => meta,
        _ => {
            result.status = "unreadable".into();
            result.notes.push("Log is missing, is not a regular file, or is a symbolic link.".into());
            return result;
        }
    };
    result.log_path = Some(path.to_string_lossy().into_owned());
    result.modified_unix_seconds = meta.modified().ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok().map(|d| d.as_secs()));
    result.log_size = Some(meta.len());
    match read_tail(path) {
        Ok((text, size, truncated)) => {
            result.log_size = Some(size);
            result.truncated = truncated;
            result.findings = parse_log(&text);
            result.status = if result.findings.is_empty() { "no-signatures" } else { "review" }.into();
            if truncated { result.notes.push("Only the final 2 MiB of this log were analyzed.".into()); }
            result.notes.push("These are pattern matches, not verified root causes. Old logs may not reflect the latest launch.".into());
        }
        Err(error) => {
            result.status = "unreadable".into();
            result.notes.push(error);
        }
    }
    result
}

#[cfg(test)]
pub(crate) fn analyze_game(home: &Path, app_id: &str, library_path: &str) -> GameDiagnostics {
    analyze_candidates(log_candidates(home, app_id), app_id, library_path)
}
fn analyze_candidates(candidates: Vec<PathBuf>, app_id: &str, library_path: &str) -> GameDiagnostics {
    let mut existing: Vec<(SystemTime, PathBuf)> = candidates.into_iter().filter_map(|p| {
        let meta = fs::symlink_metadata(&p).ok()?;
        if !meta.is_file() || meta.file_type().is_symlink() { return None; }
        Some((meta.modified().unwrap_or(UNIX_EPOCH), p))
    }).collect();
    existing.sort_by(|a, b| b.0.cmp(&a.0));
    match existing.first() {
        Some((_, path)) => analyze_exact_log(path, app_id, library_path),
        None => empty_game(app_id, library_path),
    }
}

fn read_trimmed(path: &Path, limit: u64) -> Option<String> {
    read_trimmed_from(File::open(path).ok()?, limit)
}

fn read_trimmed_from(reader: impl Read, limit: u64) -> Option<String> {
    let mut bytes = Vec::new();
    reader.take(limit.checked_add(1)?).read_to_end(&mut bytes).ok()?;
    if bytes.len() as u64 > limit { return None; }
    String::from_utf8(bytes).ok().map(|s| s.trim().into())
}

fn find_executable(name: &str) -> Option<PathBuf> {
    find_executable_in(name, &env::var_os("PATH")?)
}

fn find_executable_in(name: &str, search_path: &std::ffi::OsStr) -> Option<PathBuf> {
    use std::os::unix::ffi::OsStrExt;
    env::split_paths(search_path).map(|dir| dir.join(name)).find(|path| {
        if !path.is_file() { return false; }
        let Ok(path) = std::ffi::CString::new(path.as_os_str().as_bytes()) else { return false; };
        // Check effective credentials and ACLs; executable symlinks remain valid.
        unsafe { libc::faccessat(libc::AT_FDCWD, path.as_ptr(), libc::X_OK, libc::AT_EACCESS) == 0 }
    })
}

fn gpu_devices(root: &Path) -> Vec<String> {
    let mut devices = Vec::new();
    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if !name.starts_with("card") || !name[4..].bytes().all(|b| b.is_ascii_digit()) || name.len() == 4 { continue; }
            let device_path = entry.path().join("device");
            let Some(vendor) = read_trimmed(&device_path.join("vendor"), 128) else { continue; };
            let device = read_trimmed(&device_path.join("device"), 128).unwrap_or_else(|| "unknown".into());
            let driver = fs::read_link(device_path.join("driver")).ok()
                .and_then(|p| p.file_name().map(|s| s.to_string_lossy().into_owned())).unwrap_or_else(|| "unknown".into());
            devices.push(format!("{name}: vendor {vendor}, device {device}, driver {driver}"));
        }
    }
    devices.sort();
    devices
}

fn system_info() -> SystemInfo {
    let os = read_trimmed(Path::new("/etc/os-release"), 16_384)
        .and_then(|s| s.lines().find(|line| line.starts_with("PRETTY_NAME=")).map(|line| line.trim_start_matches("PRETTY_NAME=").trim_matches('"').into()))
        .unwrap_or_else(|| "Unknown Linux".into());
    let kernel = read_trimmed(Path::new("/proc/sys/kernel/osrelease"), 4096).unwrap_or_else(|| "Unknown".into());
    let mut icd_manifests = Vec::new();
    for directory in ["/usr/share/vulkan/icd.d", "/etc/vulkan/icd.d", "/usr/local/share/vulkan/icd.d"] {
        if let Ok(entries) = fs::read_dir(directory) {
            for entry in entries.flatten().take(128) {
                if entry.path().extension().is_some_and(|ext| ext == "json") {
                    icd_manifests.push(entry.path().to_string_lossy().into_owned());
                }
            }
        }
    }
    icd_manifests.sort();
    icd_manifests.dedup();
    SystemInfo {
        os, kernel, arch: env::consts::ARCH.into(), gpu_devices: gpu_devices(Path::new("/sys/class/drm")),
        vulkaninfo_path: find_executable("vulkaninfo").map(|p| p.to_string_lossy().into_owned()),
        icd_manifests,
        notes: vec![
            "Host ICD manifests and PCI devices do not confirm 32-bit Vulkan support, Flatpak access, or game-specific GPU selection.".into(),
            "Run the optional Vulkan probe for an active host-side check. It does not test the Steam Runtime container.".into(),
        ],
    }
}

pub fn diagnose(report: &ScanReport) -> DiagnosticsReport {
    let Some(home) = env::var_os("HOME").map(PathBuf::from) else {
        return DiagnosticsReport { games: vec![], system: system_info(), warnings: vec!["HOME is unavailable; cannot search Proton logs.".into()] };
    };
    let mut results = Vec::new();
    for game in &report.games {
        results.push(analyze_owned_game(&home, &game.app_id, &game.library_path, report));
    }
    DiagnosticsReport { games: results, system: system_info(), warnings: vec![] }
}

fn read_stream_limited<R: Read + std::os::fd::AsRawFd>(mut reader: R) -> String {
    let fd=reader.as_raw_fd();
    // Nonblocking reads make the deadline apply even when descendants retain pipes.
    let flags=unsafe { libc::fcntl(fd,libc::F_GETFL) };
    if flags<0 || unsafe { libc::fcntl(fd,libc::F_SETFL,flags|libc::O_NONBLOCK) }<0 {return "Output capture unavailable".into();}
    let deadline=std::time::Instant::now()+Duration::from_secs(6);
    let mut retained = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
        if std::time::Instant::now()>=deadline {break;}
        match reader.read(&mut buffer) {
            Err(e) if e.kind()==std::io::ErrorKind::WouldBlock=>{thread::sleep(Duration::from_millis(20));},
            Ok(0) | Err(_) => break,
            Ok(n) if retained.len() < MAX_OUTPUT => {
                let length = n.min(MAX_OUTPUT - retained.len());
                retained.extend_from_slice(&buffer[..length]);
            }
            Ok(_) => {} // Drain pipes to avoid blocking the child, but cap retained data.
        }
    }
    String::from_utf8_lossy(&retained).into_owned()
}

pub fn probe_vulkan() -> VulkanProbe {
    let Some(binary) = find_executable("vulkaninfo") else {
        return VulkanProbe { status: "unavailable".into(), exit_code: None, output: String::new(),
            detail: "vulkaninfo is not installed or is not in PATH. On Zorin/Ubuntu, the optional package is vulkan-tools.".into() };
    };
    let mut child = match crate::font_runtime::child_command(binary).arg("--summary").stdin(Stdio::null())
        .stdout(Stdio::piped()).stderr(Stdio::piped()).spawn() {
        Ok(child) => child,
        Err(e) => return VulkanProbe { status: "failed".into(), exit_code: None, output: String::new(), detail: format!("Cannot launch vulkaninfo: {e}") },
    };
    let stdout = child.stdout.take().map(|pipe| thread::spawn(move || read_stream_limited(pipe)));
    let stderr = child.stderr.take().map(|pipe| thread::spawn(move || read_stream_limited(pipe)));
    let start = std::time::Instant::now();
    let (status, code) = loop {
        match child.try_wait() {
            Ok(Some(exit)) => break (if exit.success() { "available" } else { "failed" }, exit.code()),
            Err(_) => { let _ = child.kill(); let _ = child.wait(); break ("failed", None); }
            Ok(None) if start.elapsed() >= Duration::from_secs(6) => {
                let _ = child.kill(); let _ = child.wait(); break ("timeout", None);
            }
            Ok(None) => thread::sleep(Duration::from_millis(50)),
        }
    };
    let out = stdout.and_then(|join| join.join().ok()).unwrap_or_default();
    let err = stderr.and_then(|join| join.join().ok()).unwrap_or_default();
    let combined = format!("{}\n{}", out, err);
    VulkanProbe { status: status.into(), exit_code: code, output: combined.chars().take(12_000).collect(),
        detail: match status {
            "available" => "vulkaninfo exited successfully in this host session; this does not verify per-game Vulkan support.".into(),
            "timeout" => "The Vulkan query exceeded the 6-second limit and was terminated.".into(),
            _ => "The host Vulkan query failed. Inspect its reported output and display/GPU configuration.".into(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    static SEQUENCE: AtomicUsize = AtomicUsize::new(0);
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let path = env::temp_dir().join(format!("proton-medic-diag-{}-{}", std::process::id(), SEQUENCE.fetch_add(1, Ordering::SeqCst)));
            fs::create_dir_all(&path).unwrap(); Self(path)
        }
    }
    impl Drop for Fixture { fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); } }

    #[test]
    fn executable_lookup_skips_nonexecutable_files_and_preserves_path_order() {
        use std::os::unix::fs::PermissionsExt;
        let dir = Fixture::new();
        let paths: Vec<PathBuf> = ["blocked", "first", "second"].iter().map(|name| dir.0.join(name)).collect();
        for (index, path) in paths.iter().enumerate() {
            fs::create_dir(path).unwrap();
            let binary = path.join("vulkaninfo");
            fs::write(&binary, "#!/bin/sh\nexit 0\n").unwrap();
            fs::set_permissions(&binary, fs::Permissions::from_mode(if index == 0 { 0o644 } else { 0o755 })).unwrap();
        }
        let search_path = env::join_paths(&paths).unwrap();
        assert_eq!(find_executable_in("vulkaninfo", &search_path), Some(paths[1].join("vulkaninfo")));
        assert_eq!(find_executable_in("vulkaninfo", paths[0].as_os_str()), None);
    }

    #[test]
    fn executable_lookup_accepts_symlinks_to_executable_files() {
        use std::os::unix::fs::{symlink, PermissionsExt};
        let dir = Fixture::new();
        let target = dir.0.join("utility");
        fs::write(&target, "#!/bin/sh\nexit 0\n").unwrap();
        fs::set_permissions(&target, fs::Permissions::from_mode(0o755)).unwrap();
        let binary = dir.0.join("vulkaninfo");
        symlink(&target, &binary).unwrap();
        assert_eq!(find_executable_in("vulkaninfo", dir.0.as_os_str()), Some(binary));
    }

    #[test]
    fn executable_lookup_rejects_directories_and_dangling_symlinks() {
        let dir = Fixture::new();
        fs::create_dir(dir.0.join("vulkaninfo")).unwrap();
        std::os::unix::fs::symlink(dir.0.join("missing"), dir.0.join("broken")).unwrap();
        assert_eq!(find_executable_in("vulkaninfo", dir.0.as_os_str()), None);
        assert_eq!(find_executable_in("broken", dir.0.as_os_str()), None);
    }

    #[test]
    fn small_attribute_reads_bound_actual_bytes_instead_of_metadata() {
        let mut input = std::io::Cursor::new(b"0x10de\nextra");
        assert_eq!(read_trimmed_from(&mut input, 6), None);
        assert_eq!(input.position(), 7);
        assert_eq!(read_trimmed_from(&b"0x10de\n"[..], 7).as_deref(), Some("0x10de"));
        assert_eq!(read_trimmed_from(&[255u8][..], 7), None);
        let f = Fixture::new();
        let path = f.0.join("vendor");
        fs::write(&path, "0x10de\n").unwrap();
        assert_eq!(read_trimmed(&path, 7).as_deref(), Some("0x10de"));
        // Procfs attributes expose usable content despite a zero metadata size.
        assert!(read_trimmed(Path::new("/proc/sys/kernel/osrelease"), 4096).is_some());
    }

    #[test]
    fn recognizes_independent_signatures_without_declaring_root_cause() {
        let findings = parse_log("err:module:import_dll Library MSVCP140.dll not found\nerr:vulkan: failed to create Vulkan instance\nUnhandled page fault on read access\n");
        assert!(findings.iter().any(|f| f.code == "vulkan-init"));
        assert!(findings.iter().any(|f| f.code == "wine-exception"));
        assert!(findings.iter().any(|f| f.code == "wine-module"));
        assert!(findings.iter().all(|f| !f.recommendation.is_empty()));
    }

    #[test]
    fn no_log_means_unknown_not_broken() {
        let dir = Fixture::new();
        let report = analyze_game(&dir.0, "123456", "/game-library");
        assert_eq!(report.status, "no-log");
        assert!(report.findings.is_empty());
    }

    #[test]
    fn matches_logs_only_by_validated_app_id() {
        let dir = Fixture::new();
        fs::write(dir.0.join("steam-987.log"), "vk_error_device_lost\n").unwrap();
        assert_eq!(analyze_game(&dir.0, "987", "/lib").findings[0].code, "vulkan-device-lost");
        assert!(log_candidates(&dir.0, "../987").is_empty());
        assert!(log_candidates(&dir.0, "abc").is_empty());
        assert_eq!(analyze_game(&dir.0, "456", "/lib").status, "no-log");
    }

    #[test]
    fn reads_limited_log_tail() {
        let dir = Fixture::new();
        let log = dir.0.join("steam-1.log");
        fs::write(&log, format!("header\n{}\nvk_error_initialization_failed\n", "x".repeat(LOG_TAIL_BYTES as usize))).unwrap();
        let result = analyze_game(&dir.0, "1", "/lib");
        assert!(result.truncated);
        assert!(result.findings.iter().any(|f| f.code == "vulkan-init"));
    }

    #[test]
    fn generic_noise_is_not_flagged() {
        let lines = "fixme:some message\nShader cache built successfully\nDXVK enabled\n";
        assert!(parse_log(lines).is_empty());
    }

    #[test]
    fn oversized_log_rejected_without_reading() {
        let dir = Fixture::new();
        let path = dir.0.join("steam-12.log");
        let file = File::create(&path).unwrap();
        file.set_len(MAX_LOG_BYTES + 1).unwrap();
        let result = analyze_game(&dir.0, "12", "/lib");
        assert_eq!(result.status, "unreadable");
    }

    #[test]
    fn evidence_is_sanitized_and_bounded() {
        let evidence = compact_evidence(&format!("{}\n\u{0001}", "x".repeat(300)));
        assert_eq!(evidence.chars().count(), 260);
        assert!(!evidence.contains('\n'));
    }
    #[test] fn anticheat_reference_is_not_an_unsupported_platform_claim() {
        let reference = parse_log("EasyAntiCheat initialized successfully");
        assert!(reference.iter().all(|f| f.code != "anticheat-blocked"));
        let blocked = parse_log("Easy Anti-Cheat is not supported on this platform");
        assert!(blocked.iter().any(|f| f.code == "anticheat-blocked"));
    }

    #[test] fn output_deadline_survives_an_open_descendant_pipe(){
        let (reader,_held_writer)=std::os::unix::net::UnixStream::pair().unwrap();
        let start=std::time::Instant::now();assert!(read_stream_limited(reader).is_empty());
        assert!(start.elapsed()<Duration::from_secs(8));
    }

}

// Standard Proton log locations are shared by games within one Steam installation.
// Unknown/shared libraries or multiple installations of one kind cannot establish ownership.
pub(crate) fn owned_log_candidates(home: &Path, app_id: &str, library: &str, scan: &ScanReport) -> Vec<PathBuf> {
    let Some(owner)=scan.libraries.iter().find(|l|l.path==library) else { return vec![]; };
    let kind=owner.install_kind.as_str();
    if !matches!(kind,"Native"|"Flatpak") || scan.installations.iter().filter(|i|i.kind==kind).count()>1 { return vec![]; }
    let mixed=scan.installations.iter().any(|i|i.kind!=kind);
    // A home-directory log alone does not identify its writer when both clients exist.
    if kind=="Native" && mixed { return vec![]; }
    let flatpak=home.join(".var/app/com.valvesoftware.Steam");
    log_candidates(home,app_id).into_iter().filter(|p| {
        if kind=="Flatpak" { !mixed || p.starts_with(&flatpak) } else { !p.starts_with(&flatpak) }
    }).collect()
}

pub(crate) fn analyze_owned_game(home: &Path, app_id: &str, library: &str, scan: &ScanReport) -> GameDiagnostics {
    let candidates=owned_log_candidates(home,app_id,library,scan);
    if candidates.is_empty() {
        let mut result=empty_game(app_id,library);
        result.notes.push("Log ownership is ambiguous for this Steam installation. Shared logs are excluded from automatic repairs.".into());
        return result;
    }
    analyze_candidates(candidates,app_id,library)
}
