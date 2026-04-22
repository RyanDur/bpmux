use std::process;

fn main() {
    if let Err(e) = bpmux::run() {
        eprintln!("Error: {e}");
        process::exit(1);
    }
}
