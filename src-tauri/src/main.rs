#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
fn main() {
    if let Some(code)=proton_medic_lib::windows_shortcuts::handle_cli(std::env::args().collect()){std::process::exit(code);}
    proton_medic_lib::run();
}
