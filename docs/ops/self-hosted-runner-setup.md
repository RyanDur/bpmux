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

## Slice-2 hybrid migration (#1252)

Slice-1 chose macOS arm64 substrate; slice-2 grounding revealed
existing `ci.yml` jobs all target `ubuntu-latest` with Linux-specific
deps (xvfb-run, `npx playwright install --with-deps`). Pure
runs-on-directive lift-and-shift breaks on OS mismatch.

**Smallest-first hybrid path** chosen (per dev-lane owner-call at
2026-05-10 22:00 CST + `feedback_authority_defer_on_named_pattern`):

| Job | Pre-#1252 | Post-#1252 step 1 | Reason |
|-----|-----------|-------------------|--------|
| `build-bpmux` | ubuntu-latest | ubuntu-latest (unchanged) | builds Linux x64 binary used by Playwright e2e |
| `test` (fmt/clippy/cargo test/audit) | ubuntu-latest | `[self-hosted, macos, arm64]` | no GUI, no xvfb, cargo cross-platform |
| `playwright-e2e` | ubuntu-latest | ubuntu-latest (unchanged) | xvfb-run is Linux-only; sharding-rethink needed for single-runner |

**Trade-offs:**

- ✓ Real-workload (cargo build + test + clippy + audit) validates
  bpmux-mac-m1 beyond synthetic smoke
- ✓ Quota-bypass: one heavy job migrated; reduces GHA-hosted minutes
- ✓ Linux coverage preserved for build-artifact + e2e (Playwright
  shards still parallelize across GHA-hosted runners)
- ✗ Cargo target cache rebuilds for aarch64-apple-darwin (first run)
- ⚠ Resource contention with Ryan's live work on the same Mac (M1 Pro
  32GB; cargo test ≈ 1.5GB peak; should be fine but watch)

**Deferred (sister-story or slice 1.5):**

- `playwright-e2e` migration — requires xvfb-replacement (macOS-native
  Playwright headed mode) OR Linux substrate provisioning (Docker on
  Mac, or separate Linux self-hosted runner)
- `build-bpmux` migration — only worth migrating if e2e also migrates;
  otherwise produces wrong-arch artifact for Linux Playwright shards
