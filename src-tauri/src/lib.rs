#[tauri::command]
fn startup_diagnostic(message:String){
    let safe:String=message.chars().filter(|c|!c.is_control() || *c=='\n').take(4000).collect();
    eprintln!("[Proton Medic startup] {safe}");
}
mod font_runtime;
mod scanner;
mod workflow;
mod log_monitor;
mod game_logging;
mod windows_games;
mod diagnostics;
mod repairs;
mod advisor;
mod auto_select;
mod launch_test;

#[tauri::command]
async fn scan_steam() -> Result<scanner::ScanReport, String> {
    tauri::async_runtime::spawn_blocking(scanner::scan)
        .await
        .map_err(|error| format!("Scanner thread failed: {error}"))
}

#[tauri::command]
async fn analyze_diagnostics() -> Result<diagnostics::DiagnosticsReport, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let scanned = scanner::scan();
        diagnostics::diagnose(&scanned)
    }).await.map_err(|error| format!("Diagnostics worker failed: {error}"))
}

#[tauri::command]
async fn probe_vulkan() -> Result<diagnostics::VulkanProbe, String> {
    tauri::async_runtime::spawn_blocking(diagnostics::probe_vulkan)
        .await.map_err(|error| format!("Vulkan probe worker failed: {error}"))
}

#[tauri::command]
async fn preview_repair(app_id: String, library_path: String, action: String) -> Result<repairs::RepairPlan, String> {
    tauri::async_runtime::spawn_blocking(move || repairs::plan(app_id, library_path, action))
        .await.map_err(|e| e.to_string())?
}
#[tauri::command]
async fn apply_repair(app_id: String, library_path: String, action: String, confirmed: bool) -> Result<repairs::RepairOutcome, String> {
    tauri::async_runtime::spawn_blocking(move || repairs::apply(app_id, library_path, action, confirmed))
        .await.map_err(|e| e.to_string())?
}
#[tauri::command]
async fn list_repair_backups(app_id: String, library_path: String) -> Result<Vec<repairs::RepairBackup>, String> {
    tauri::async_runtime::spawn_blocking(move || repairs::list_backups(app_id, library_path))
        .await.map_err(|e| e.to_string())?
}
#[tauri::command]
async fn restore_repair(app_id: String, library_path: String, action: String, backup_name: String, confirmed: bool) -> Result<repairs::RepairOutcome, String> {
    tauri::async_runtime::spawn_blocking(move || repairs::restore(app_id, library_path, action, backup_name, confirmed))
        .await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn recommend_proton() -> Result<advisor::AdvisorReport, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let scan = scanner::scan();
        let report = diagnostics::diagnose(&scan);
        advisor::recommend(&scan, &report)
    }).await.map_err(|e| format!("Proton advisor worker failed: {e}"))
}

#[tauri::command]
async fn auto_select_proton(app_id: String, library_path: String, confirmed: bool) -> Result<auto_select::AutoSelection, String> {
    tauri::async_runtime::spawn_blocking(move || auto_select::select(app_id, library_path, confirmed))
        .await.map_err(|e| format!("Auto-select worker failed: {e}"))?
}


#[tauri::command]
async fn test_game_launch(app_id: String, library_path: String, seconds: u64, confirmed: bool) -> Result<launch_test::LaunchResult, String> {
    tauri::async_runtime::spawn_blocking(move || launch_test::test_game(app_id, library_path, seconds, confirmed))
        .await.map_err(|e| format!("Launch test worker failed: {e}"))?
}

#[tauri::command]
async fn plan_game_fix(app_id: String, library_path: String) -> Result<workflow::FixPlan, String> {
    tauri::async_runtime::spawn_blocking(move || workflow::plan(&app_id, &library_path)).await.map_err(|e| e.to_string())?
}
#[tauri::command]
async fn start_game_fix(app_id: String, library_path: String, expected_tool: String, confirmed: bool) -> Result<workflow::FixStarted, String> {
    tauri::async_runtime::spawn_blocking(move || workflow::start(app_id, library_path, expected_tool, confirmed)).await.map_err(|e| e.to_string())?
}
#[tauri::command]
async fn test_game_fix(session_id: String, confirmed: bool) -> Result<launch_test::LaunchResult, String> {
    tauri::async_runtime::spawn_blocking(move || workflow::test(&session_id, confirmed)).await.map_err(|e| e.to_string())?
}
#[tauri::command]
async fn rollback_game_fix(session_id: String, confirmed: bool) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || workflow::rollback(&session_id, confirmed)).await.map_err(|e| e.to_string())?
}
#[tauri::command]
async fn finish_game_fix(session_id: String, confirmed: bool) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || workflow::finish(&session_id, confirmed)).await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn monitor_game_logs(enabled: bool) -> Result<Vec<log_monitor::Update>, String> {
    tauri::async_runtime::spawn_blocking(move || log_monitor::poll(enabled)).await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn prepare_game_logging(app_id: String, library_path: String, confirmed: bool) -> Result<game_logging::Status, String> {
    tauri::async_runtime::spawn_blocking(move || game_logging::prepare(app_id, library_path, confirmed)).await.map_err(|e| e.to_string())?
}
#[tauri::command]
async fn get_game_logging(app_id: String, library_path: String) -> Result<Option<game_logging::Status>, String> {
    tauri::async_runtime::spawn_blocking(move || game_logging::status(&app_id, &library_path)).await.map_err(|e| e.to_string())?
}
#[tauri::command]
async fn restore_game_logging(app_id: String, library_path: String, confirmed: bool) -> Result<game_logging::Status, String> {
    tauri::async_runtime::spawn_blocking(move || game_logging::restore(app_id, library_path, confirmed)).await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn list_windows_games() -> Result<windows_games::Library, String> {
    tauri::async_runtime::spawn_blocking(windows_games::list).await.map_err(|e| e.to_string())?
}
#[tauri::command]
async fn install_windows_game(name: String, installer: String, runtime_id: String, confirmed: bool) -> Result<windows_games::Game, String> {
    tauri::async_runtime::spawn_blocking(move || windows_games::install(name, installer, runtime_id, confirmed)).await.map_err(|e| e.to_string())?
}
#[tauri::command]
async fn select_windows_game_executable(id: String, path: String, confirmed: bool) -> Result<windows_games::Game, String> {
    tauri::async_runtime::spawn_blocking(move || windows_games::select_executable(id, path, confirmed)).await.map_err(|e| e.to_string())?
}
#[tauri::command]
async fn launch_windows_game(id: String, confirmed: bool) -> Result<windows_games::Game, String> {
    tauri::async_runtime::spawn_blocking(move || windows_games::launch(id, confirmed)).await.map_err(|e| e.to_string())?
}
#[tauri::command]
async fn analyze_windows_game(id: String) -> Result<windows_games::LogReport, String> {
    tauri::async_runtime::spawn_blocking(move || windows_games::findings(id)).await.map_err(|e| e.to_string())?
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    eprintln!("[Proton Medic startup] Native process 0.6.6 starting");
    let _font_runtime=match font_runtime::initialize(){Ok(guard)=>guard,Err(error)=>{eprintln!("[Proton Medic startup] Font isolation failed: {error}");return;}};
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|_|{eprintln!("[Proton Medic startup] Tauri setup completed");Ok(())})
        .on_page_load(|_,payload|{eprintln!("[Proton Medic startup] Page event: {:?}",payload.event());})
        .invoke_handler(tauri::generate_handler![startup_diagnostic,list_windows_games, install_windows_game, select_windows_game_executable, launch_windows_game, analyze_windows_game, prepare_game_logging, get_game_logging, restore_game_logging, monitor_game_logs, plan_game_fix, start_game_fix, test_game_fix, rollback_game_fix, finish_game_fix, scan_steam, analyze_diagnostics, probe_vulkan, preview_repair, apply_repair, list_repair_backups, restore_repair, recommend_proton, auto_select_proton, test_game_launch])
        .run(tauri::generate_context!())
        .expect("failed to start Proton Medic");
}
