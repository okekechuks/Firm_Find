// Selected explicitly with Cargo --bin; never rename the ordinary application binary.
fn main() {
    if let Err(error) = obtainer_lib::run_native_test() {
        eprintln!("OBTAINER native test refused: {error}");
        std::process::exit(1);
    }
}
