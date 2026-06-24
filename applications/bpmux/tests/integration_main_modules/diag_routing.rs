#![allow(clippy::unwrap_used, clippy::expect_used)]

use bpmux::diag::{DiagSink, MemoryDiagSink, clear_thread_sink, install_thread_sink, log_diag};
use std::rc::Rc;

#[test]
fn log_diag_writes_to_the_injected_sink_not_the_real_log() {
    let marker = format!(
        "DIAG-ROUTING-MARKER-{}",
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
    );

    let sink = Rc::new(MemoryDiagSink::new());
    install_thread_sink(sink.clone());
    log_diag(&marker);
    clear_thread_sink();

    let captured = sink.read();
    assert!(
        captured.contains(&marker),
        "log_diag must write the marker to the injected sink; got: {captured:?}"
    );

    log_diag("AFTER-CLEAR-MUST-NOT-REACH-THE-INJECTED-SINK");
    assert_eq!(
        sink.read(),
        captured,
        "clearing the thread sink must detach it — later log_diag calls must not append to it"
    );
}
