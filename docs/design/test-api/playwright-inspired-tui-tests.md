# Playwright-inspired TUI test API for bpmux

*Designer research — adapted from Playwright's locator, assertion, and action patterns for an 80x24 terminal TUI.*

## Problem

Current tests mix user intent with implementation mechanics: raw `wait_for_tui`, manual `thread::sleep`, and bare `screen.contains()`. Tests describe HOW to poll the screen rather than WHAT the user sees.

## Playwright patterns adapted for TUI

### 1. Locators → region-scoped text queries

Playwright: `page.getByRole('button', { name: 'Submit' })`
bpmux: `tui.region(Region::Panel).to_show("READY")`

A TUI has no DOM but it has REGIONS (status bar, chat history, input line, panel). Locators become region-scoped text queries.

### 2. Auto-retrying assertions

Playwright: `expect(locator).toHaveText()` retries until timeout
bpmux: `tui.expect(Region::Chat).to_show("message")` — combines waiting + asserting, auto-retries with 5s default timeout, produces clear failure with screen snapshot

### 3. Actions as user gestures

```
tui.run_command("/board")     — types command + waits for readiness
tui.press("Esc")              — sends key + waits for screen to settle
tui.receive_message("dev", "hello")  — injects test data via REST
```

## Human-side test examples

```rust
#[test]
fn board_opens_and_closes() {
    let tui = Tui::start();
    tui.run_command("/board");
    tui.expect(Region::Panel).to_show("READY");
    tui.expect(Region::Input).to_show(">");
    tui.press("Esc");
    tui.expect(Region::Panel).not_to_show("READY");
}

#[test]
fn focus_filters_messages_by_role() {
    let tui = Tui::start();
    tui.receive_message("dev", "dev-says-alpha");
    tui.receive_message("designer", "designer-says-gamma");
    tui.run_command("/focus dev");
    tui.expect(Region::Chat).to_show("dev-says-alpha");
    tui.expect(Region::Chat).not_to_show("designer-says-gamma");
    tui.expect(Region::StatusBar).to_show("[focus: dev]");
}

#[test]
fn command_palette_filters_and_executes() {
    let tui = Tui::start();
    tui.press("Ctrl+K");
    tui.expect(Region::Panel).to_show("actions");
    tui.type_into_palette("who");
    tui.press("Enter");
    tui.expect(Region::Chat).to_show("Participants");
}
```

## Agent-side test examples (MCP analog)

```rust
#[test]
fn agent_sees_board_via_mcp() {
    let bridge = Bridge::connect();
    let board = bridge.call("view_board");
    expect(&board).to_contain_story(191, "delivered");
    expect(&board).not_to_contain_story(191, "accepted");
}

#[test]
fn topic_delivery_filters_by_subscription() {
    let bridge = Bridge::connect();
    bridge.call("send_message", text: "secret", topic: "teaching");
    
    let subscriber = bridge.messages_as("designer", topics: ["teaching"]);
    expect(&subscriber).to_contain("secret");
    
    let outsider = bridge.messages_as("outsider");
    expect(&outsider).not_to_contain("secret");
}

#[test]
fn lane_enforcement_rejects_wrong_role() {
    let bridge = Bridge::connect_as("designer");
    let result = bridge.call("reorder_story", number: 191, position: 1);
    expect(&result).to_be_rejected("reorder is restricted to pm");
}
```

## What changes from current tests

| Current | Playwright-inspired |
|---|---|
| `wait_for_tui("text", timeout)` + `assert!(screen.contains())` | `tui.expect(Region::Chat).to_show("text")` |
| `type_line("/board")` + `sleep` + manual screen check | `tui.run_command("/board")` |
| `screen.contains("READY")` anywhere on screen | `tui.region(Region::Panel).to_show("READY")` — scoped |
| Bare `post_chat("from", "text")` | `tui.receive_message("from", "text")` — semantic |
| No agent-side test vocabulary | `bridge.call()` + `expect().to_contain()` |

## Two-audience coverage

Every feature test should have BOTH:
- A human-side test (`Tui` — what the screen shows)
- An agent-side test (`Bridge` — what the MCP tool returns)

Same behavior verified from both perspectives. Both use the same action → assert pattern. Both include negative tests.
