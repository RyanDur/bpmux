use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tokio::sync::broadcast;

use super::paste_mode::PasteModeSniffer;

pub(super) fn spawn_output_reader(
    mut reader: Box<dyn std::io::Read + Send>,
    output: broadcast::Sender<Vec<u8>>,
    paste_capable: Arc<AtomicBool>,
    current_paste_mode: Arc<AtomicBool>,
) {
    std::thread::spawn(move || {
        use std::io::Read;
        let mut sniffer = PasteModeSniffer::new();
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => {
                    let _ = output.send(Vec::new());
                    break;
                }
                Ok(n) => {
                    note_paste_mode(&mut sniffer, &buf[..n], &paste_capable, &current_paste_mode);
                    let _ = output.send(buf[..n].to_vec());
                }
                Err(_) => {
                    let _ = output.send(Vec::new());
                    break;
                }
            }
        }
    });
}

fn note_paste_mode(
    sniffer: &mut PasteModeSniffer,
    chunk: &[u8],
    paste_capable: &AtomicBool,
    current_paste_mode: &AtomicBool,
) {
    let Some(state) = sniffer.observe(chunk) else {
        return;
    };
    if state {
        paste_capable.store(true, Ordering::Relaxed);
    }
    current_paste_mode.store(state, Ordering::Relaxed);
}
