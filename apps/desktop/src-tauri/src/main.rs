#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if let Err(error) = obtainer_lib::run() {
        obtainer_lib::startup::report_failure(&error);
        std::process::exit(1);
    }
}
