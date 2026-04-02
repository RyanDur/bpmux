#![deny(unsafe_code)]

pub mod session;

pub fn run() -> anyhow::Result<()> {
    println!("bpmux v{}", env!("CARGO_PKG_VERSION"));
    Ok(())
}
