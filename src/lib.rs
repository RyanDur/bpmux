#![deny(unsafe_code)]

pub fn run() -> anyhow::Result<()> {
    println!("bpmux v{}", env!("CARGO_PKG_VERSION"));
    Ok(())
}
