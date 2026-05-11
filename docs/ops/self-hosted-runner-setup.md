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

## Slice-2 hybrid migration (#1252) — transitional shape

Per Ryan-directive 2026-05-11 03:07 UTC ("keep migrating everything
to the self hosted runner"), initial impl migrated all 3 jobs.
Empirical CI on f9ebe47 / c8ba878 / 5bdb235 showed playwright-e2e
on macOS arm64 hits per-test 30-90s timeouts (~3× Linux+xvfb
baseline) because each test spawns its own bpmux backend with
filesystem-backed state — concurrent Electron+bpmux launches
serialize on bpmux's startup cost.

Ryan-call 2026-05-11 06:08 CST: the proper fix is **datastore
dependency-injection** — refactor bpmux's storage layer so tests
can inject an in-memory store, eliminating per-test
backend-startup latency. Filed as sister-story under Epic #1247.

**Until sister-story lands, ci.yml is hybrid:**

| Job | Current state | Reason |
|-----|---------------|--------|
| `build-bpmux` | `ubuntu-latest` | produces Linux x64 binary for the Linux-hosted Playwright matrix |
| `test` (fmt/clippy/nextest/audit) | `[self-hosted, macos, arm64]` | empirical 2-3 min wall-clock; zero failures across 4+ runs; cache warm via Swatinem |
| `playwright-e2e` (6-shard) | `ubuntu-latest` | requires Store-trait DI substrate (sister-story) before macOS-native migration; full quota-bypass deferred |

**Empirical evidence motivating the revert:**

- f9ebe47 (all 3 jobs migrated; 6-shard matrix on single self-hosted runner): 3/5 completed shards FAILED with 30s timeouts; projected total wall-clock 5-6 hours
- c8ba878 (--workers=8 + 90s testTimeout): 11/~306 tests still timed out at 90s
- 5bdb235 (--workers=4 + 30s default testTimeout): 25 failures (worse, because lower timeout exposed more per-test slowness)
- Cumulative finding: per-test slowness on macOS arm64 + Electron is real (not just concurrency-contention); requires substrate-architecture fix, not CI-config tuning

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
