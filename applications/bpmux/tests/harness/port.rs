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

pub fn wait_for_port_bindable(port: u16, deadline: std::time::Duration) -> bool {
    let start = std::time::Instant::now();
    while start.elapsed() < deadline {
        if TcpListener::bind(("127.0.0.1", port)).is_ok() {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    false
}
