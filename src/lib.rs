pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    println!("bpmux v{}", env!("CARGO_PKG_VERSION"));
    Ok(())
}
