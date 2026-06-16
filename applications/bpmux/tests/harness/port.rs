//! Test-time port allocation.

use std::net::TcpListener;

/// Binds to a free OS-assigned port, returns its number, and releases the
/// listener immediately. Tests use this to isolate from each other and from
/// any running bpmux on the developer machine.
pub fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("bind to free port")
        .local_addr()
        .expect("local addr")
        .port()
}
