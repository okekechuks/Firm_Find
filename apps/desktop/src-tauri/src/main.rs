#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if let Err(error) = obtainer_lib::run() {
        eprintln!("OBTAINER could not start: {error}");
        std::process::exit(1);
    }
}
