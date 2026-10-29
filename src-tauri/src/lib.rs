mod coordination;
mod proton_manager;
mod support_report;
mod game_icons;
mod app_update;
mod portable;
pub mod windows_shortcuts;
#[tauri::command]
async fn list_proton_versions()->Result<proton_manager::Local,String>{tauri::async_runtime::spawn_blocking(proton_manager::local).await.map_err(|e|e.to_string())}
#[tauri::command]
async fn fetch_proton_releases(page:u32)->Result<Vec<proton_manager::Release>,String>{proton_manager::catalog(page).await}
#[tauri::command]
fn install_proton_version(tag:String,root:String,confirmed:bool)->Result<(),String>{proton_manager::start(tag,root,confirmed)}
#[tauri::command]
fn cancel_proton_download()->Result<(),String>{proton_manager::cancel()}
#[tauri::command]
async fn list_game_protons(app_id:String,library_path:String)->Result<Vec<scanner::CompatibilityTool>,String>{
    tauri::async_runtime::spawn_blocking(move||auto_select::manual_tools(&app_id,&library_path)).await.map_err(|e|e.to_string())?
}
#[tauri::command]
async fn select_game_proton(app_id:String,library_path:String,tool_path:String,confirmed:bool)->Result<auto_select::AutoSelection,String>{
    tauri::async_runtime::spawn_blocking(move||{let _activity=app_update::activity()?;auto_select::select_manual(app_id,library_path,tool_path,confirmed)}).await.map_err(|e|e.to_string())?
}
#[tauri::command]
fn startup_diagnostic(message:String){
    let safe:String=message.chars().filter(|c|!c.is_control() || *c=='\n').take(4000).collect();
    eprintln!("[Proton Medic startup] {safe}");
}
mod font_runtime;
mod scanner;
mod inventory;
mod runtime_session;
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
async fn prepare_support_report(options: support_report::Options) -> Result<support_report::Preview, String> {
    tauri::async_runtime::spawn_blocking(move || support_report::prepare(options)).await.map_err(|e|e.to_string())?
}
#[tauri::command]
async fn save_support_report(app: tauri::AppHandle, token: u64, confirmed: bool) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || support_report::save(app, token, confirmed)).await.map_err(|e|e.to_string())?
}
#[tauri::command]
async fn scan_steam() -> Result<scanner::ScanReport, String> {
    tauri::async_runtime::spawn_blocking(||{let scan=inventory::refresh();game_icons::remember_steam(&scan);(*scan).clone()})
        .await
        .map_err(|error| format!("Scanner thread failed: {error}"))
}

#[tauri::command]
async fn analyze_diagnostics() -> Result<diagnostics::DiagnosticsReport, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let scanned = inventory::steam();
        (*inventory::analysis(&scanned)).clone()
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
    tauri::async_runtime::spawn_blocking(move || { let _activity = app_update::activity()?; repairs::apply(app_id, library_path, action, confirmed) })
        .await.map_err(|e| e.to_string())?
}
#[tauri::command]
async fn list_repair_backups(app_id: String, library_path: String) -> Result<Vec<repairs::RepairBackup>, String> {
    tauri::async_runtime::spawn_blocking(move || repairs::list_backups(app_id, library_path))
        .await.map_err(|e| e.to_string())?
}
#[tauri::command]
async fn restore_repair(app_id: String, library_path: String, action: String, backup_name: String, confirmed: bool) -> Result<repairs::RepairOutcome, String> {
    tauri::async_runtime::spawn_blocking(move || { let _activity = app_update::activity()?; repairs::restore(app_id, library_path, action, backup_name, confirmed) })
        .await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn recommend_proton() -> Result<advisor::AdvisorReport, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let scan = inventory::steam();
        let report = inventory::analysis(&scan);
        advisor::recommend(&scan, &report)
    }).await.map_err(|e| format!("Proton advisor worker failed: {e}"))
}

#[tauri::command]
async fn auto_select_proton(app_id: String, library_path: String, confirmed: bool) -> Result<auto_select::AutoSelection, String> {
    tauri::async_runtime::spawn_blocking(move || { let _activity = app_update::activity()?; auto_select::select(app_id, library_path, confirmed) })
        .await.map_err(|e| format!("Auto-select worker failed: {e}"))?
}


#[tauri::command]
async fn test_game_launch(app_id: String, library_path: String, seconds: u64, confirmed: bool) -> Result<launch_test::LaunchResult, String> {
    tauri::async_runtime::spawn_blocking(move || { let _activity = app_update::activity()?; launch_test::test_game(app_id, library_path, seconds, confirmed) })
        .await.map_err(|e| format!("Launch test worker failed: {e}"))?
}

#[tauri::command]
async fn plan_game_fix(app_id: String, library_path: String) -> Result<workflow::FixPlan, String> {
    tauri::async_runtime::spawn_blocking(move || workflow::plan(&app_id, &library_path)).await.map_err(|e| e.to_string())?
}
#[tauri::command]
async fn start_game_fix(app_id: String, library_path: String, expected_tool: String, confirmed: bool) -> Result<workflow::FixStarted, String> {
    tauri::async_runtime::spawn_blocking(move || { let _activity = app_update::activity()?; workflow::start(app_id, library_path, expected_tool, confirmed) }).await.map_err(|e| e.to_string())?
}
#[tauri::command]
async fn test_game_fix(session_id: String, confirmed: bool) -> Result<launch_test::LaunchResult, String> {
    tauri::async_runtime::spawn_blocking(move || { let _activity = app_update::activity()?; workflow::test(&session_id, confirmed) }).await.map_err(|e| e.to_string())?
}
#[tauri::command]
async fn rollback_game_fix(session_id: String, confirmed: bool) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || { let _activity = app_update::activity()?; workflow::rollback(&session_id, confirmed) }).await.map_err(|e| e.to_string())?
}
#[tauri::command]
async fn finish_game_fix(session_id: String, confirmed: bool) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || { let _activity = app_update::activity()?; workflow::finish(&session_id, confirmed) }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn monitor_game_logs(enabled: bool) -> Result<Vec<log_monitor::Update>, String> {
    tauri::async_runtime::spawn_blocking(move || { let Ok(_activity) = app_update::activity() else { return Ok(vec![]); }; log_monitor::poll(enabled) }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn prepare_game_logging(app_id: String, library_path: String, confirmed: bool) -> Result<game_logging::Status, String> {
    tauri::async_runtime::spawn_blocking(move || { let _activity = app_update::activity()?; game_logging::prepare(app_id, library_path, confirmed) }).await.map_err(|e| e.to_string())?
}
#[tauri::command]
async fn get_game_logging(app_id: String, library_path: String) -> Result<Option<game_logging::Status>, String> {
    tauri::async_runtime::spawn_blocking(move || game_logging::status(&app_id, &library_path)).await.map_err(|e| e.to_string())?
}
#[tauri::command]
async fn restore_game_logging(app_id: String, library_path: String, confirmed: bool) -> Result<game_logging::Status, String> {
    tauri::async_runtime::spawn_blocking(move || { let _activity = app_update::activity()?; game_logging::restore(app_id, library_path, confirmed) }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn list_windows_games() -> Result<windows_games::Library, String> {
    tauri::async_runtime::spawn_blocking(windows_games::list).await.map_err(|e| e.to_string())?
}
#[tauri::command]
async fn install_windows_game(name: String, installer: String, runtime_id: String, confirmed: bool) -> Result<windows_games::Game, String> {
    tauri::async_runtime::spawn_blocking(move || { let _activity = app_update::activity()?; windows_games::install(name, installer, runtime_id, confirmed) }).await.map_err(|e| e.to_string())?
}
#[tauri::command]
async fn select_windows_game_executable(id: String, path: String, confirmed: bool) -> Result<windows_games::Game, String> {
    tauri::async_runtime::spawn_blocking(move || { let _activity = app_update::activity()?; windows_games::select_executable(id, path, confirmed) }).await.map_err(|e| e.to_string())?
}
#[tauri::command]
async fn launch_windows_game(id: String, confirmed: bool) -> Result<windows_games::Game, String> {
    tauri::async_runtime::spawn_blocking(move || { let _activity = app_update::activity()?; windows_games::launch(id, confirmed) }).await.map_err(|e| e.to_string())?
}
#[tauri::command]
async fn sample_windows_resources(id:String,run_id:String)->Result<windows_games::monitor::Report,String>{
    tauri::async_runtime::spawn_blocking(move||windows_games::monitor::sample(id,run_id)).await.map_err(|e|e.to_string())?
}
#[tauri::command]
async fn stop_windows_resource_monitor(id:String,run_id:String)->Result<(),String>{
    tauri::async_runtime::spawn_blocking(move||windows_games::monitor::stop(id,run_id)).await.map_err(|e|e.to_string())?
}
#[tauri::command]
async fn get_windows_launch_history(id: String) -> Result<windows_games::history::History, String> {
    tauri::async_runtime::spawn_blocking(move || windows_games::history::list(id)).await.map_err(|e|e.to_string())?
}
#[tauri::command]
async fn analyze_windows_game(id: String) -> Result<windows_games::LogReport, String> {
    tauri::async_runtime::spawn_blocking(move || windows_games::findings(id)).await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn register_windows_game(name:String,path:String,prefix:String,runtime_id:String,confirmed:bool)->Result<windows_games::Game,String>{
    tauri::async_runtime::spawn_blocking(move || { let _activity = app_update::activity()?; windows_games::register(name,path,prefix,runtime_id,confirmed) }).await.map_err(|e|e.to_string())?
}
#[tauri::command]
async fn save_windows_launch_settings(id:String,value:windows_games::LaunchSettings,expected:windows_games::LaunchSettings,confirmed:bool)->Result<windows_games::Game,String>{
    tauri::async_runtime::spawn_blocking(move||{let _activity=app_update::activity()?;windows_games::save_launch_settings(id,value,expected,confirmed)}).await.map_err(|e|e.to_string())?
}
#[tauri::command]
async fn reset_windows_launch_settings(id:String,expected:windows_games::LaunchSettings,confirmed:bool)->Result<windows_games::Game,String>{
    tauri::async_runtime::spawn_blocking(move||{let _activity=app_update::activity()?;windows_games::reset_launch_settings(id,expected,confirmed)}).await.map_err(|e|e.to_string())?
}
#[tauri::command]
async fn review_windows_installation(id:String)->Result<windows_games::InstallationReview,String>{
    tauri::async_runtime::spawn_blocking(move||windows_games::review_installation(id)).await.map_err(|e|e.to_string())?
}
#[tauri::command]
async fn confirm_windows_installation(id:String,name:String,path:String,confirmed:bool)->Result<windows_games::Game,String>{
    tauri::async_runtime::spawn_blocking(move||{let _activity=app_update::activity()?;windows_games::confirm_installation(id,name,path,confirmed)}).await.map_err(|e|e.to_string())?
}
#[tauri::command]
async fn get_windows_executable_icon(id:String,path:String)->Result<Option<String>,String>{
    tauri::async_runtime::spawn_blocking(move||windows_games::preview_executable_icon(id,path)).await.map_err(|e|e.to_string())?
}
#[tauri::command]
async fn find_windows_game_executables(id:String)->Result<windows_games::ExecutableSearch,String>{
    tauri::async_runtime::spawn_blocking(move||windows_games::find_executables(id)).await.map_err(|e|e.to_string())?
}
#[tauri::command]
async fn rename_windows_game(id:String,name:String,confirmed:bool)->Result<windows_games::Game,String>{
    tauri::async_runtime::spawn_blocking(move || { let _activity = app_update::activity()?; windows_games::rename(id,name,confirmed) }).await.map_err(|e|e.to_string())?
}
#[tauri::command]
async fn create_windows_game_shortcut(id:String,confirmed:bool)->Result<Vec<String>,String>{
    tauri::async_runtime::spawn_blocking(move || { let _activity = app_update::activity()?; windows_shortcuts::create(id,confirmed) }).await.map_err(|e|e.to_string())?
}

#[tauri::command]
async fn set_windows_game_hidden(id:String,hidden:bool,confirmed:bool)->Result<(),String>{
    tauri::async_runtime::spawn_blocking(move || { let _activity = app_update::activity()?; windows_games::set_hidden(id,hidden,confirmed) }).await.map_err(|e|e.to_string())?
}
#[tauri::command]
async fn get_windows_game_icon(id:String)->Result<Option<String>,String>{
    tauri::async_runtime::spawn_blocking(move||windows_games::icon(id)).await.map_err(|e|e.to_string())?
}
#[tauri::command]
async fn get_steam_game_icon(app_id:String,library_path:String)->Result<Option<String>,String>{
    tauri::async_runtime::spawn_blocking(move||game_icons::steam_icon(app_id,library_path)).await.map_err(|e|e.to_string())?
}

#[tauri::command]
async fn begin_app_update() -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(app_update::begin).await.map_err(|e| e.to_string())?
}
#[tauri::command]
fn cancel_app_update() -> Result<(), String> { app_update::cancel() }
#[tauri::command]
async fn restart_after_update(app: tauri::AppHandle, version: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || app_update::restart(app, version)).await.map_err(|e| e.to_string())?
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    app_update::initialize();
    eprintln!("[Proton Medic startup] Native process {} starting", env!("CARGO_PKG_VERSION"));
    let _font_runtime=match font_runtime::initialize(){Ok(guard)=>guard,Err(error)=>{eprintln!("[Proton Medic startup] Font isolation failed: {error}");return;}};
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|_|{eprintln!("[Proton Medic startup] Tauri setup completed");Ok(())})
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if let Err(reason) = app_update::prepare_close() {
                    api.prevent_close();
                    use tauri_plugin_dialog::DialogExt;
                    window.dialog().message(format!("Keep Proton Medic open until active work finishes.\n\n{reason}"))
                        .title("Proton Medic is busy").show(|_| {});
                }
            }
        })
        .on_page_load(|_,payload|{eprintln!("[Proton Medic startup] Page event: {:?}",payload.event());})
        .invoke_handler(tauri::generate_handler![list_game_protons,list_proton_versions,fetch_proton_releases,install_proton_version,cancel_proton_download,select_game_proton,prepare_support_report,save_support_report,sample_windows_resources,stop_windows_resource_monitor,get_windows_launch_history,save_windows_launch_settings,reset_windows_launch_settings,review_windows_installation,confirm_windows_installation,get_windows_executable_icon,begin_app_update,cancel_app_update,restart_after_update,startup_diagnostic,set_windows_game_hidden,get_windows_game_icon,get_steam_game_icon,register_windows_game,find_windows_game_executables,rename_windows_game,create_windows_game_shortcut,list_windows_games, install_windows_game, select_windows_game_executable, launch_windows_game, analyze_windows_game, prepare_game_logging, get_game_logging, restore_game_logging, monitor_game_logs, plan_game_fix, start_game_fix, test_game_fix, rollback_game_fix, finish_game_fix, scan_steam, analyze_diagnostics, probe_vulkan, preview_repair, apply_repair, list_repair_backups, restore_repair, recommend_proton, auto_select_proton, test_game_launch])
        .run(tauri::generate_context!())
        .expect("failed to start Proton Medic");
}
