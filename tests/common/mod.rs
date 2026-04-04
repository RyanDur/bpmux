use ratatui_testlib::ScreenState;
use std::io::Read;
use std::time::{Duration, Instant};

pub fn read_until_contains(
    reader: &mut dyn Read,
    target: &str,
    timeout: Duration,
) -> (String, Vec<u8>) {
    let mut all_bytes = Vec::new();
    let mut buf = [0u8; 4096];
    let start = Instant::now();

    loop {
        if start.elapsed() >= timeout {
            break;
        }
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                all_bytes.extend_from_slice(&buf[..n]);
                let mut screen = ScreenState::new(80, 24);
                screen.feed(&all_bytes);
                if screen.contents().contains(target) {
                    break;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(_) => break,
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    let mut screen = ScreenState::new(80, 24);
    screen.feed(&all_bytes);
    (screen.contents(), all_bytes)
}
