# Spawn-isolation discipline for tests

## Rule

**Tests that spawn `bpmux` (Rust binary) MUST construct the child env explicitly via an allow-list. Never spread `process.env` (TypeScript) or inherit by default (Rust).**

This applies wherever a test boots a real bpmux subprocess:

- Rust integration tests via `applications/bpmux/tests/harness/pty.rs::TestBpmux::start`
- Playwright Electron tests via `applications/electron/tests/helpers/launch.ts::launchIsolatedElectron`

## Why

`bpmux` reads its operational config from environment variables — the
canonical list is in `applications/bpmux/src/bridge/tracker.rs::select_from_env`:

- `BPMUX_REPO`
- `BPMUX_GH_BASE_URL`
- `GH_TOKEN`
- `GITHUB_TOKEN`

When a test inherits the developer's shell env, those variables silently route the spawned bpmux's `ci_watcher` + `ingest_poll` + `hydration_loop` against **real** `api.github.com`. The test author thinks they're hitting an in-memory mock; the spawned process is hitting GitHub. This is the substrate-bug that drove the 2026-05-06 rate-limit cooldown — Ryan's GH user 939595 hit 5000/hr against api.github.com from Playwright runs that nobody intended to make external calls.

A deny-list ("strip these 4 known-dangerous keys") protects the **current** call sites. The moment `tracker.rs` learns to read a fifth env var, deny-list-based tests silently re-introduce the leak. An allow-list ("inherit only these keys; everything else is explicit") inverts the trust model — new env vars on the bpmux side require an explicit opt-in on the test side.

## How

### TypeScript (Playwright)

`applications/electron/tests/helpers/launch.ts` exports `SAFE_INHERITED_ENV_KEYS` + `pickSafeInheritedEnv(env)`. The allow-list contains the OS + Electron + Node essentials needed for spawn (PATH, HOME, TMPDIR, ELECTRON_RUN_AS_NODE, DISPLAY, etc) and **nothing bpmux reads**.

```ts
const app = await electron.launch({
  args: [...],
  env: {
    ...pickSafeInheritedEnv(process.env),
    BPMUX_SESSION_BASE: sessionBase,
    BPMUX_BACKEND_PORT: String(backendPort),
    // ... all bpmux config set explicitly
  }
})
```

Tests that need a specific bpmux env var must pass it explicitly via `extraEnv`. The helper auto-spawns an in-process mock GH server when `BPMUX_GH_BASE_URL` isn't supplied, so no test ever talks to real GitHub.

### Rust (PTY harness)

`applications/bpmux/tests/harness/pty.rs::TestBpmux::start` uses `portable_pty::CommandBuilder::new(bin)` which **does not inherit env by default**. Each required key is set explicitly via `cmd.env(k, v)`. The same allow-list discipline applies — but the language already enforces it.

## Adding a new env var

If `bpmux` learns to read a new env var (extending `select_from_env` or adding a feature flag):

1. **Decide where it should be set in tests.** Test-time only? Add to `extraEnv` explicitly per-test.
2. **System-level need?** Add to `SAFE_INHERITED_ENV_KEYS` (TS) and update `pty.rs` to forward it conditionally.
3. **Never** add it to the deny-list class of fix; that's the failure mode this discipline exists to prevent.

## Sister discipline

- `feedback_no_real_github_in_tests` (lessons) — absolute rule from Ryan 2026-05-05.
- `feedback_test_harness_must_apply_default_isolation` — saved generative lesson on isolation as a default.
- `feedback_local_pass_ci_fail_hidden_axis_coupling` — env inheritance is a hidden axis when tests pass locally on a clean shell but fail on a developer's machine.
