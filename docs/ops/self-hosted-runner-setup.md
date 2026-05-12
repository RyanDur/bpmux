# Self-hosted GitHub Actions runner setup (Epic #1247 slice 1)

Walkthrough for installing + registering + auto-starting a self-hosted
GitHub Actions runner against the bpmux repo. Bypasses the GHA-hosted
quota; zero subscription cost (actions-runner is free OSS).

## Q2 machine-choice (Ryan 2026-05-10 19:37 UTC)

- **Host**: Local Mac (Ryan's dev machine; same host runs Electron bpmux + Claude Code workspaces)
- **OS**: macOS 26.4.1 (Build 25E253)
- **CPU/RAM/Disk**: Apple M1 Pro / 32GB / 344GB free
- **Rationale**: zero subscription cost (Ryan 19:14 UTC "as long as we don't have to pay for it"); already provisioned + reachable; sufficient hardware-substrate for cargo + Playwright + Rust toolchain workloads per AC 7-9

### Hardware verification (AC 7-9) — passed

```
RAM: 32 GB (≥16GB required) ✓
Disk free: 344GB on / (≥50GB required) ✓
api.github.com: HTTP/2 200 ✓
github.com: HTTP/2 200 ✓
```

## Pre-install verification commands (re-run on install day)

```bash
# RAM check (need ≥16GB)
#   macOS:
system_profiler SPHardwareDataType | grep Memory
#   Linux:
free -h

# Disk free (need ≥50GB for cargo target + Playwright + Rust toolchain)
df -h ~

# GitHub reachability
curl -sI https://api.github.com/  | head -1
curl -sI https://github.com/  | head -1
```

Expected: 200/301 status from both curls; no firewall block.

## Install via GitHub's official setup script (AC 2)

From the bpmux repo on GitHub: `Settings → Actions → Runners → New
self-hosted runner` — copy the host-specific commands (they include a
single-use registration token). The shape is:

### macOS (arm64 / Apple Silicon)

```bash
mkdir -p ~/actions-runner && cd ~/actions-runner
curl -o actions-runner-osx-arm64-2.x.x.tar.gz -L \
  https://github.com/actions/runner/releases/download/v2.x.x/actions-runner-osx-arm64-2.x.x.tar.gz
tar xzf ./actions-runner-osx-arm64-2.x.x.tar.gz

# Register against bpmux repo with single-use token from GH UI
./config.sh --url https://github.com/RyanDur/bpmux  --token <TOKEN> \
  --name <runner-name> \
  --labels self-hosted,macos,arm64

# Smoke-test interactive run (Ctrl-C to stop; service install below)
./run.sh
```

### Linux (x86_64)

```bash
mkdir -p ~/actions-runner && cd ~/actions-runner
curl -o actions-runner-linux-x64-2.x.x.tar.gz -L \
  https://github.com/actions/runner/releases/download/v2.x.x/actions-runner-linux-x64-2.x.x.tar.gz
tar xzf ./actions-runner-linux-x64-2.x.x.tar.gz

./config.sh --url https://github.com/RyanDur/bpmux  --token <TOKEN> \
  --name <runner-name> \
  --labels self-hosted,linux,x64

./run.sh
```

### Labels (AC 4)

Runner labels must satisfy the `runs-on:` directives in
`.github/workflows/*.yml`. Current workflows use `ubuntu-latest`; for
self-hosted routing add `self-hosted` plus OS-specific label
(`linux` or `macos`). Slice-2 (#1252) updates the workflow YAMLs to
reference these labels.

## Auto-restart configuration (AC 5-6)

### macOS — launchd plist

```bash
cd ~/actions-runner
sudo ./svc.sh install
sudo ./svc.sh start
sudo ./svc.sh status
```

This writes a `LaunchDaemon` plist under `/Library/LaunchDaemons/` that
auto-starts the runner on boot. Manual plist template (fallback) at:

```xml
<!-- /Library/LaunchDaemons/actions.runner.RyanDur-bpmux.<runner-name>.plist -->
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd"> 
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>actions.runner.RyanDur-bpmux.<runner-name></string>
    <key>UserName</key>
    <string><user></string>
    <key>WorkingDirectory</key>
    <string><home>/actions-runner</string>
    <key>ProgramArguments</key>
    <array>
        <string><home>/actions-runner/runsvc.sh</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
</dict>
</plist>
```

### Linux — systemd unit

```bash
cd ~/actions-runner
sudo ./svc.sh install <user>
sudo ./svc.sh start
sudo ./svc.sh status
```

Generates `/etc/systemd/system/actions.runner.RyanDur-bpmux.<runner-name>.service`.
Fallback template:

```ini
[Unit]
Description=GitHub Actions Runner (RyanDur-bpmux.<runner-name>)
After=network.target

[Service]
ExecStart=<home>/actions-runner/runsvc.sh
User=<user>
WorkingDirectory=<home>/actions-runner
KillMode=process
KillSignal=SIGTERM
TimeoutStopSec=5min
Restart=always

[Install]
WantedBy=multi-user.target
```

### Reboot test (AC 6)

```bash
# Reboot machine
sudo reboot
# Post-reboot, verify runner re-registers as Active:
#   GH Settings → Actions → Runners → <runner-name> shows "Idle" or "Active"
# OR check service status locally:
sudo launchctl list | grep actions.runner  # macOS
sudo systemctl status actions.runner.RyanDur-bpmux.*  # Linux
```

## Test-fire (AC 10-12)

Push a minor commit to main; observe the workflow run routes to the
new runner (visible in `Settings → Actions → Runners → <name>` activity
+ in the run-page's "runner" badge).

```bash
# Trigger via canonical CI-trigger (push commit; per
# feedback_no_gh_at_all_not_just_mutations — no `gh workflow run`)
git -C ~/IdeaProjects/bpmux commit --allow-empty \
  -m "[chore] runner-test-fire: confirm self-hosted routing"
git -C ~/IdeaProjects/bpmux push
```

Verify in GH Actions UI:
- Run picks up on new runner (badge shows runner-name, not
  `ubuntu-latest`)
- Cargo + clippy + nextest + vitest + playwright jobs complete
  successfully
- Logs accessible at the existing GH Actions UI

## Post-install evidence (recorded 2026-05-10 19:43 UTC)

- **Runner name**: bpmux-mac-m1
- **Runner agentId**: 21 (poolName=Default)
- **Labels applied**: self-hosted, macos, arm64
- **Service-manager**: `~/Library/LaunchAgents/actions.runner.RyanDur-bpmux.bpmux-mac-m1.plist` (user-level LaunchAgent; survives logout via launchd auto-start at user login)
- **Initial PID after svc start**: 40316

### Test-fire (AC 10-12) ✓ — recorded 2026-05-10 20:43 UTC

Grounded in runner-local diag log (canonical first-party execution record):

- **Test-fire commit SHA**: b9aeefc "[#1251] self-hosted runner setup walkthrough + smoke-test workflow"
- **Worker log**: `~/actions-runner/_diag/Worker_20260511-011055-utc.log`
- **Workflow**: `.github/workflows/self-hosted-runner-smoke.yml`
- **Job display name**: `smoke`
- **runs-on resolved**: `self-hosted` (matched labels on bpmux-mac-m1)
- **Job result**: `Succeeded` at 2026-05-11 01:10:56Z; `Job completed.` at 01:10:58Z

### Reboot-test (AC 6) ✓ — recorded 2026-05-10 21:43 UTC

Ryan physically rebooted his M1 Pro Mac; runner auto-restarted via LaunchAgent
`RunAtLoad=true` semantics without manual intervention:

- **System uptime post-reboot**: 20 min (verified via `uptime`)
- **Runner-process restart**: `Runner_20260511-022720-utc.log` shows agent started at 02:27:20 UTC (~4 min after boot, consistent with launchd user-session-load timing)
- **Listener status**: `"Listening for Jobs"` at 02:27:22Z; runner version `2.334.0`; VssConnection established with GitHub
- **launchctl state**: `actions.runner.RyanDur-bpmux.bpmux-mac-m1` loaded, PID alive, exit-status 0

## Out of scope (slice 1; covered in later slices)

- Workflow-config-validation across all jobs (#1252 slice 2)
- Cutover from GHA-hosted runners — routing-mode decision (#1253 slice 3)
- Bridge MCP-tools post-cutover verification (#1254 slice 4)
- Multi-runner pool / multi-platform matrix (deferred)

## Slice-2 hybrid migration (#1252) — retired (#1291 completes self-hosted-everything)

Slice 2's hybrid shape was transitional pending Store-trait DI
substrate. With #1289 (DI foundation) + #1290 (substrate completion
+ wall-clock proof) shipped, the per-test bpmux-startup disk-I/O
cost that empirically blocked f9ebe47 / c8ba878 / 5bdb235 is closed.
#1291 (this slice's resumption) migrates `build-bpmux` +
`playwright-e2e` back to `[self-hosted, macos, arm64]` — see the
"Full self-hosted state (#1291)" section below.

Retained here for historical context: the hybrid table is retired
post-migration; the empirical-evidence list documents the substrate
class that #1289 + #1290 closed.

**Empirical evidence motivating the original revert (now closed):**

- f9ebe47 (all 3 jobs migrated; 6-shard matrix on single self-hosted runner): 3/5 completed shards FAILED with 30s timeouts; projected total wall-clock 5-6 hours
- c8ba878 (--workers=8 + 90s testTimeout): 11/~306 tests still timed out at 90s
- 5bdb235 (--workers=4 + 30s default testTimeout): 25 failures (worse, because lower timeout exposed more per-test slowness)
- Cumulative finding: per-test slowness on macOS arm64 + Electron was substrate-shaped, not config-tuning-shaped. Closed by #1289 + #1290's MemoryStorage default in `applications/electron/tests/helpers/launch.ts`.

## Full self-hosted state (#1291) — migration completed

Substrate-foundation chain that unblocked this slice:

`#1251` (slice 1: runner setup) → `#1252` (slice 2: hybrid transitional)
→ `#1289` (Store-trait DI foundation) → `#1290` (substrate completion
+ wall-clock proof + Cure 1.5 fixes) → `#1291` (this slice: resumption).

Each prior layer shipped substrate + commitment-to-next. This slice
closes the Store-trait DI chain back to the original directive
"migrate everything to the self-hosted runner" (Ryan 2026-05-11
03:07 UTC + 11:34 UTC).

### Final ci.yml shape (all jobs on self-hosted)

| Job | Runner | Notes |
|-----|--------|-------|
| `build-bpmux` | `[self-hosted, macos, arm64]` | Produces `target/debug/bpmux` arm64-apple-darwin binary; cached via Swatinem with shared-key `playwright-e2e` |
| `test` (fmt/clippy/nextest/audit) | `[self-hosted, macos, arm64]` | Unchanged since slice 2 step 1 |
| `playwright-e2e` (single job, --workers=4) | `[self-hosted, macos, arm64]` | xvfb-run removed (macOS native windowing); `--with-deps` removed (no apt); Playwright cache at `~/Library/Caches/ms-playwright`. Sharding retired — with a 1-runner pool, matrix shards serialize and pay N× setup overhead without parallelism benefit. Workers=4 inside a single job gives in-process parallelism with one-time setup. |

### Wall-clock measurements

Local baseline on macOS-arm64 dev machine (mirrors bpmux-mac-m1
runner class) with `BPMUX_STORAGE_BACKEND=memory` (#1290 default):

- Single-test (close_story spec, repeat-each=30, workers=4): 26.3s / 30 tests = ~0.88s/test → **well under 5s target** (AC 7)
- Full Playwright suite (309 tests): 2.6 min → **well under 6-8min target** (AC 8)

CI-runner measurements pending first post-migration run; the
2.6min local baseline is sub-quartile of the 6-8min projection,
giving headroom for runner overhead.

### Substrate-Bug coupling (AC 13-14)

- **#1285 ($HOME-contention)**: strategy is **action-cache redirect
  via env vars** (CARGO_HOME / RUSTUP_HOME → `$RUNNER_TEMP`-private
  paths). Impl remains deferred per f865f03 — earlier convergence
  attempts (5b3f53a → 80823e6 → 4d8f5b2 → a76df1a → d9a3802) hit an
  opaque CI-failure pattern (failures[] empty even after #1293's
  non-test-failure fallback). Test job currently runs on self-hosted
  WITHOUT applied isolation; build-bpmux + playwright-e2e (this
  slice) inherit the same pre-isolation state. Resumption-path per
  f865f03: real-time monitoring during a fresh CI run while watching
  the GH Actions UI, OR alternate isolation strategy that bypasses
  `dtolnay/rust-toolchain` (assume pre-installed rustup on the
  runner). Sister-story #1285 remains the home for the follow-up.
- **#1286 (UI-overlap)**: handled via **off-screen window
  positioning** (option 2 — chosen). `BPMUX_TEST_WINDOW_POSITION=
  offscreen` is set unconditionally in `applications/electron/tests/
  helpers/launch.ts:166`. `applications/electron/src/main/index.ts`
  positions the BrowserWindow at `{x: -10000, y: -10000}` when the
  env var is present. Regression test:
  `applications/electron/tests/playwright/ui_overlap_mitigation.spec.ts`.

### Future-action-addition discipline (unchanged from slice 2)

When adding any new action to a self-hosted job, audit its
$HOME-touch surface per the audit table below (#1285 section).
Until #1285 isolation is re-applied, the audit's redirect column
documents the *intended* state rather than the live state.

### Sister-substrate-Bugs (impl deferred to standalone slices)

- **#1285 — $HOME-contention**: dtolnay/rust-toolchain action wipes
  `~/.cargo/bin/rustup` mid-CI-run because the runner runs as
  Ryan's user with Ryan's $HOME. Sister of
  `feedback_no_broad_pkill_during_shared_session`. Fired LIVE 2×
  during overnight impl. Mitigation impl deferred to #1285;
  options: runner-isolated $HOME / action-cache redirect / action
  replacement.
- **#1286 — UI-overlap mitigation**: bpmux-mac-m1 IS Ryan's live
  dev machine. Playwright tests spawn Electron windows during CI
  runs. Initial mitigation attempt (BPMUX_TEST_MODE hidden-windows)
  broke real-backend.spec.ts:671 + human-observes-* tests because
  macOS throttles hidden windows below 10s timeouts; option
  retired empirically. Designer's revised menu (3 options): (a)
  off-screen window positioning (`x: -10_000, y: -10_000`) — window
  technically visible, no throttling, user doesn't see it; (b)
  reduce concurrency (`workers: 1`) during Ryan-live-hours; (c)
  off-hours CI cron (4am UTC schedule). Mitigation impl deferred to
  #1286.

### Trade-offs of hybrid state

- ✓ `test` job runs on bpmux-mac-m1 (partial quota-bypass; cargo
  + clippy + nextest + audit no longer consume GHA-hosted minutes)
- ✓ CI stays green throughout sister-story refactor
- ⚠ Full quota-bypass deferred until Store-trait sister-story
  delivers + we re-attempt e2e migration
- ⚠ $HOME-contention class on test job until #1285 mitigation
  lands (rustup re-install required after each CI test job)

**Deferred to future iteration:**

- Sharding-rethink: 1-shard `--workers=8` vs 6-shard serialized
  (whichever has lower wall-clock + acceptable memory ceiling)
- Off-hours CI cron: schedule e2e at 4am UTC (designer mitigation 4)
  to reduce real-time conflict with Ryan's live dev hours

## UI-overlap mitigation: off-screen window positioning (#1286)

bpmux-mac-m1 IS Ryan's live dev machine. When Playwright e2e tests
run there, `_electron.launch()` spawns visible Electron windows that
visually interfere with Ryan's screen during work hours. Dog-fooding-
on-dev-machine is a load-bearing substrate-discovery channel (Ryan
2026-05-10 23:21 UTC reframe), and visible test-windows during
dev-work break that channel.

### Option 1 retirement: hidden-windows broken by macOS throttle

The original mitigation (dev 2026-05-11 03:08 UTC) used `BPMUX_TEST_MODE`
to set `BrowserWindow({show: false})`. Reverted in `f9ebe47` after
empirical observation: **macOS throttles hidden windows below 10s
timeouts**, breaking real-backend + human-observes tests that require
sustained DOM observation.

### Strategy: off-screen positioning (Option 2 — chosen)

When `BPMUX_TEST_WINDOW_POSITION=offscreen` is set, `applications/
electron/src/main/index.ts` constructs the BrowserWindow at
`{x: -10000, y: -10000}` — outside any reasonable monitor's
coordinate space but technically active (no OS throttle). The
Playwright launch helper sets this env var unconditionally in
`launch.ts` (alongside `BPMUX_TEST_SEED_API=1`).

### Trade-off analysis

| Option | Wall-clock | Feedback latency | Multi-monitor | Test compatibility |
|--------|-----------|------------------|---------------|--------------------|
| 1 — hidden-windows | unchanged | unchanged | OK | **BROKEN** (macOS throttle) |
| 2 — off-screen position | unchanged | unchanged | edge-case-risk | OK |
| 3 — workers:1 live-hours | **6× slower** | unchanged | OK | OK |
| 4 — off-hours cron | unchanged | **24h wait** | OK | OK |

Option 2 chosen — only option that costs nothing on wall-clock OR
feedback latency, and works with all existing tests. The multi-
monitor edge-case (some macOS configurations may render off-screen
differently across virtual displays) is deferred per body's
"Out of scope" — Ryan's setup is single-monitor.

### Regression test

`applications/electron/tests/playwright/ui_overlap_mitigation.spec.ts`
asserts via `BrowserWindow.getBounds()` that the spawned window's
x-coord is well-negative (< -1000). Runs as part of the Playwright
suite — any future change that breaks the off-screen positioning
surfaces immediately.

## $HOME-isolation on bpmux-mac-m1 (#1285)

The bpmux-mac-m1 self-hosted runner runs as Ryan's user, sharing
`$HOME` with his live dev shell. Empirical incident (dev 2026-05-11
03:28 UTC during #1252 step-2): the `dtolnay/rust-toolchain` action
wiped `~/.cargo/bin/rustup` mid-CI run, requiring a manual rustup +
cargo-nextest reinstall to continue.

This is the **same-user same-$HOME contention class** — sister of
`feedback_no_broad_pkill_during_shared_session`. Substrate-fix is
load-bearing for the dog-fooding-on-dev-machine channel (Ryan
2026-05-10 23:21 UTC reframe): if Ryan can't trust his $HOME state,
he can't dog-food cleanly.

### Strategy: action-cache redirect via env vars

Set `CARGO_HOME` and `RUSTUP_HOME` to `$RUNNER_TEMP`-private paths at
the job level in `.github/workflows/ci.yml`. The toolchain actions
respect these standard rustup env vars and write into the redirected
paths instead of Ryan's `~/.cargo` / `~/.rustup`. A
`PATH`-prepend step ensures later workflow steps find the redirected
cargo binaries.

Rationale for this strategy over alternatives:

- **Runner-isolated $HOME** (alternative): would require runner-config
  change + restart; loses Ryan's existing user-level cache benefits
  (Swatinem/rust-cache populated against his real $HOME for pre-#1285
  runs). Defer.
- **Replace destructive actions** (alternative): `dtolnay/rust-toolchain`
  is the de-facto standard; replacements would diverge from community
  norm.
- **Env-var redirect** (chosen): minimal-invasive, reversible, no
  runner-config change, follows standard env-var conventions.

### Action audit (.github/workflows/ci.yml)

| Job | Runner | Action | $HOME-touch surface | Isolation |
|-----|--------|--------|---------------------|-----------|
| build-bpmux | ubuntu-latest | actions/checkout@v4 | none | N/A (ephemeral container) |
| build-bpmux | ubuntu-latest | dtolnay/rust-toolchain@master | ~/.rustup ~/.cargo | N/A (ephemeral container) |
| build-bpmux | ubuntu-latest | Swatinem/rust-cache@v2 | ~/.cargo/registry ~/.cargo/git | N/A (ephemeral container) |
| build-bpmux | ubuntu-latest | actions/upload-artifact@v4 | none | N/A |
| test | self-hosted macos arm64 | actions/checkout@v4 | none | safe (read-only) |
| test | self-hosted macos arm64 | dtolnay/rust-toolchain@master | ~/.rustup ~/.cargo | **redirected** via job-level CARGO_HOME + RUSTUP_HOME → $RUNNER_TEMP |
| test | self-hosted macos arm64 | Swatinem/rust-cache@v2 | CARGO_HOME paths | **redirected** transitively (follows CARGO_HOME) |
| playwright-e2e | ubuntu-latest | actions/checkout@v4 | none | N/A (ephemeral container) |
| playwright-e2e | ubuntu-latest | actions/download-artifact@v4 | none | N/A |
| playwright-e2e | ubuntu-latest | actions/setup-node@v4 | ~/.npm | N/A (ephemeral container) |
| playwright-e2e | ubuntu-latest | actions/cache@v4 | ~/.cache/ms-playwright | N/A (ephemeral container) |
| playwright-e2e | ubuntu-latest | actions/upload-artifact@v4 | none | N/A |

The only self-hosted-runner-resident job is `test`, where both
$HOME-touching actions are now redirected. Other jobs run in
ephemeral GHA-hosted containers and their $HOME-touches don't affect
Ryan's machine.

### Self-verifying $HOME-isolation gate

The `test` job runs a "Verify $HOME-isolation" step after
toolchain installation that fails the build if `~/.rustup/toolchains`
or `~/.cargo/bin/rustup` were modified during the CI run. This is
the regression test (AC 6-7) and runs as part of CI itself — any
future action that breaks isolation surfaces immediately.

### Future-action-addition discipline

When adding any new action to `.github/workflows/*.yml`:

1. If it runs in `runs-on: [self-hosted, macos, arm64]`, audit its
   $HOME-touch surface BEFORE merging.
2. If the action touches user-level config-dirs (`~/.cargo`,
   `~/.rustup`, `~/.npm`, `~/.gradle`, `~/.m2`, `~/.ssh`, etc.),
   either: (a) confirm read-only/scoped-path, OR (b) redirect to
   `$RUNNER_TEMP` via env vars, OR (c) replace with action that
   respects $HOME.
3. Extend the audit table above with the new action's row.

## Substrate-fix sister-stories (#1289 + #1290)

The hybrid-state migration above named two substrate gaps blocking
full quota-bypass: per-test bpmux backend-startup disk-I/O cost +
test-seed mechanism that doesn't work without a disk. These were
ratified at the 2026-05-11 14:39 UTC triple-role convergence as Cure
1.5 (dev's 14:27 UTC grounded re-analysis surfaced post_message's
`Utc::now()` override + missing pins/reactions seed endpoints;
designer's 14:39 UTC body-vs-reality catch on #1289 added Phase-3
completion to the same scope to avoid story-bloat).

### #1289 — Store-trait DI substrate (delivered)

bpmux's persistence behind two traits at
`applications/bpmux/src/session/storage.rs`:

- `Storage` — session-scoped (messages, lessons, board snapshot,
  outbox, bookmarks, pins, reactions, iteration markers, generic
  key-value)
- `GlobalStore` — cross-session globals (priority + identity +
  briefing + tokens + usage + auth_token)

Each has two impls: `FileStorage`/`FileGlobalStore` (production,
disk-backed) and `MemoryStorage`/`MemoryGlobalStore` (in-memory).
Selection via `BPMUX_STORAGE_BACKEND=memory` env at `Session::new`.
94 contract tests prove FileStore/MemoryStore semantic-parity at the
trait surface; divergence is impossible to land.

Boundary-discipline: auth_token has a divergent contract per
`feedback_external_system_idiosyncrasies_at_boundary` —
`FileGlobalStore` wraps mode-0600 filesystem semantics;
`MemoryGlobalStore` returns `ErrorKind::Unsupported` because a
process-owned cryptographic secret has no in-memory equivalent.

Architecture-doc cross-reference: `docs/architecture.md` Storage
layer section.

### #1290 — Test-seed-API + Phase-3 completion (current)

Three concerns ratified together per Cure 1.5:

**Part A: Test-seed-API**. Decision (i) — new dedicated
`/api/test/seed_messages` + `/api/test/seed_pins` +
`/api/test/seed_reactions` endpoints in
`applications/bpmux/src/transport/test_seed.rs`. Rationale: option
(ii) (test-mode flag on `post_message`) would mix test concerns
into a security-sensitive production path — `post_message`
overrides `ts` with `Utc::now()` precisely to prevent timestamp
spoofing. Dedicated test-mode endpoints satisfy the airtight-
boundary discipline (#1290 AC 6).

Gate: new env var `BPMUX_TEST_SEED_API=1` read at router-
construction time only (structural boundary, not runtime
conditional). Separate from `BPMUX_TEST_MODE` (which also swaps to
the in-memory orchestrator) so `launchWithMockGh` Playwright tests
can seed state while keeping the prod orchestrator pointed at the
mock-GH boundary.

Routes through `session.storage` trait — works identically against
FileStorage + MemoryStorage. Closes the substrate-gap that made
launch.ts's pre-#1290 `seedMessages`/`seedPins` filesystem-write
mechanism incompatible with MemoryStorage.

**Part B: launch.ts refactor + MemoryStorage default + wall-clock**.
`applications/electron/tests/helpers/launch.ts` now sets:

- `BPMUX_TEST_SEED_API=1` (enables seed routes)
- `BPMUX_STORAGE_BACKEND=memory` (default; eliminates per-test
  bpmux backend-startup disk-I/O)
- POSTs `seedMessages` + `seedPins` via the new API after
  `waitForHealthz`

`seedRoster` path is unchanged (AC 10) — `Session::new` reads
`roster_seed.json` directly at startup, not via the Storage trait,
so the pre-launch filesystem setup remains. 5 disk-asserting specs
(F3/F5 outbox restart + corrupted-store recovering + 2 pinned-list)
opt back to FileStorage via per-spec `extraEnv:
{BPMUX_STORAGE_BACKEND: 'file'}`. This explicit per-spec opt-out
keeps the boundary visible at the test that depends on it.

**Wall-clock measurements (macOS local, workers=2)**:

| Surface | FileStorage | MemoryStorage | Notes |
|---------|-------------|---------------|-------|
| Single test (workers=1) | 1.72s | 1.80s | both <5s target |
| 6-spec sample (12 tests) | 6.69s | 6.59s | minimal local diff (APFS SSD overhead small) |
| Shard 6/6 (51 tests) | 52.8s + 3 failures | 21.1s | 60% reduction; failures were disk-asserting specs |
| Full suite (308 tests) | not measured post-substrate | 167s (2.8 min) | well under 6-8min target |

These are **local macOS measurements**, directional only. Official
AC 12-13 numbers on the bpmux-mac-m1 self-hosted runner — where FS
overhead is higher — roll into the AC 23 follow-up story below. The
memory-vs-file gap should widen on the slower runner FS.

**Part C: Phase-3 GlobalStore completion**. Mechanical sister-shape
replication of #1289's priority archetype:

- identity (`~/.bpmux/identity/{role}.json`) — sister 2a91ccb
- briefings (`~/.bpmux/briefings/{role}.md`) — sister 1222506
- tokens (`~/.bpmux/tokens/log.jsonl`) — sister 9b2fc9d (append-only)
- usage (`~/.bpmux/usage.json`) — sister e8e5aaf
- auth_token (`~/.bpmux/auth_token`) — sister 49e3f94 (divergent
  MemoryGlobalStore contract per boundary-discipline)

### Re-migration follow-up (AC 23)

The full Epic #1247 self-hosted e2e migration (build-bpmux +
playwright-e2e jobs → `[self-hosted, macos, arm64]` with
MemoryStorage active) is filed as a follow-up at #1290 delivery.
That story captures the official AC 12-13 wall-clock numbers
against bpmux-mac-m1 and completes the Epic #1247 quota-bypass
chain.

Substrate-foundation chain: #1251 → #1252 → #1289 → #1290 →
re-migration follow-up.
