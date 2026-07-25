use std::process;

fn main() {
    if let Err(e) = composition_root::run() {
        eprintln!("Error: {e}");
        process::exit(1);
    }
}
