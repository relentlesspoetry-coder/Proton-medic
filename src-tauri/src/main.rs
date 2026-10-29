#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
fn main() {
    if std::env::args_os().len() == 2 && std::env::args_os().nth(1).is_some_and(|arg| arg == "--proton-medic-version") {
        println!("{}", env!("CARGO_PKG_VERSION")); return;
    }
    if let Some(code)=proton_medic_lib::runtime_session::handle_cli(){std::process::exit(code);}
    if let Some(code)=proton_medic_lib::windows_shortcuts::handle_cli(std::env::args().collect()){std::process::exit(code);}
    proton_medic_lib::run();
}
