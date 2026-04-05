//! Test harness for bpmux integration tests, organized by bounded context.
//!
//! - `port` — dynamic port allocation for test isolation
//! - `pty` — pseudo-terminal helpers and `TestBpmux`
//! - `bridge` — `TestBridge` and `NotificationCapture`

#![allow(dead_code)]

pub mod bridge;
pub mod port;
pub mod pty;
