# CI migration — Concourse vs alternatives

*Designer research-spike per #1248 / Ryan 2026-05-09 18:36 UTC. Output for Epic #1247 CI migration. Trigger: GitHub Actions free-tier quota exhausted in 1.5 weeks of bpmux's velocity. Need self-hosted or lower-cost CI substrate.*

## Problem framing

bpmux's CI usage exceeds GHA free-tier quota. Each push fires:
- 1× build-bpmux job (Rust toolchain + cargo build + artifact upload)
- 1× test job (cargo fmt + clippy + cargo test + cargo audit)
- 6× Playwright shards (Electron build + xvfb + Playwright browser install + 4 workers per shard)

Total: 8 concurrent jobs × push frequency × 1.5 weeks = quota exhaustion. The work isn't optional — pre-push gates require all 8 to be green; substrate is appropriate for the team's quality-paramount stance.

Constraint: substrate must continue to support today's gates (cargo fmt/clippy/test/audit + Playwright matrix-shards under xvfb + Rust binary artifact-passing + parallel execution). Cannot relax pre-push discipline.

## Current GHA workflow profile (what to replicate)

`.github/workflows/ci.yml`:
- **Triggers**: push to main + pull_request + workflow_call
- **Toolchain**: Rust 1.95.0 via dtolnay/rust-toolchain + Swatinem/rust-cache
- **Test substrate**: cargo + Node 20 + Playwright + xvfb-run
- **Matrix**: Playwright shards 1-6 with workers=4
- **Artifact pass**: bpmux binary build → playwright shards download
- **Browser cache**: ~/.cache/ms-playwright keyed on package-lock.json
- **Failure artifacts**: Playwright traces uploaded on failure

Total surface: 1 workflow file; ~115 lines; mature + working.

## Concourse-CI deep dive

[Concourse](https://concourse-ci.org/) — OSS (Apache 2.0); pull-based; container-only; self-hosted.

**Strengths:**
- Pipeline-as-code via declarative YAML (sister-shape to GHA + Drone)
- Strong artifact-passing semantics ("inputs" / "outputs" between jobs)
- Real-time pipeline visualization (good for debugging multi-stage flows)
- Resources are first-class (git-resource, time-resource, semver-resource, etc.) — pulls trigger jobs
- Stateless workers; Docker-only execution

**Weaknesses for bpmux's case:**
- **Substantial paradigm shift** from GHA push-trigger model. Concourse uses RESOURCE-driven pulls; semantics differ from "on push run X." Migration cost HIGH.
- **Operational substrate**: requires PostgreSQL + Web component + Worker components. Self-host means running 3+ services.
- **No native macOS/Windows worker** — Linux containers only. (Not blocking for bpmux today; but limits flexibility.)
- **Higher learning curve** — fly CLI + custom resource-types + pipeline-set semantics differ from GHA familiarity.
- **Steeper than necessary for single-team use case** — Concourse shines at large-team / multi-pipeline-orchestration; bpmux is one pipeline.

**Cost**: free OSS; operational-cost is self-host infrastructure (1 box minimum; PostgreSQL + Web + Worker; ~$5-20/month if hosted on cloud VM; $0 if home-server).

## Alternative platforms

### Self-hosted GitHub Actions runner (LOWEST migration cost)

[GitHub-hosted runners → self-hosted runners](https://docs.github.com/en/actions/hosting-your-own-runners). Keep `.github/workflows/ci.yml` as-is; route runs to local-machine runner.

**Strengths:**
- **ZERO migration cost** — existing workflow files run unchanged
- **Same logs / UI / triggers** as GHA today
- bpmux quota-exhaustion completely bypassed (self-hosted minutes don't count against free tier)
- Setup: 1 command (`./config.sh`) on a Linux/Mac box; agent registers with GitHub

**Weaknesses:**
- Single-machine concurrency limit (one runner per box; matrix shards serialize unless multiple runners)
- Box must stay online + reachable
- Security: jobs run as the runner-user; needs sandboxing if running untrusted PRs

**Cost**: $0 software + electricity for hosted box; could reuse Ryan's dev machine OR small dedicated NUC ($200-500 one-time).

### Buildkite (LOW migration cost; hybrid model)

[Buildkite](https://buildkite.com/) — SaaS controller + self-hosted runners (called "agents"). YAML pipeline config.

**Strengths:**
- **Hybrid**: build-orchestration is SaaS (no infra to host); compute is self-hosted (your hardware = your cost)
- Free for OSS / individuals (3 agents)
- Mature platform; excellent artifact + parallelism support
- Native xvfb / browser-test support
- Native Mac/Linux/Windows agent support
- Lower-cost-per-minute than CircleCI/GHA paid

**Weaknesses:**
- Requires migrating workflow files from GHA YAML to Buildkite YAML (different shape; multi-step pipeline syntax)
- Two-vendor model (GitHub source-of-truth + Buildkite CI; webhook glue between)
- Still requires self-hosted agent for compute

**Cost**: free for individual/OSS (3 agents); $20/seat for paid plans.

### Drone (MEDIUM migration cost; OSS self-hosted)

[Drone](https://www.drone.io/) — OSS (Apache 2.0); self-hosted; container-based.

**Strengths:**
- YAML config (.drone.yml); per-step container model
- Lighter than Concourse (single-binary controller; container-runner); easier ops
- Strong GitHub integration (webhook-driven push/PR)
- Pipeline-as-code matches GHA familiarity

**Weaknesses:**
- Drone's owner Harness has put it in maintenance-mode; community fork is Woodpecker (next entry)
- Migration from GHA → Drone YAML is non-trivial (different step-shape)
- Self-host = same operational cost as Concourse but lighter

**Cost**: free OSS; self-host infra (single box).

### Woodpecker (MEDIUM migration cost; OSS Drone-fork; active development)

[Woodpecker](https://woodpecker-ci.org/) — community-fork of Drone; active OSS development; Go-based; forge-agnostic.

**Strengths:**
- **Active development** vs Drone's maintenance-mode
- Same config-format as Drone (easy if migrating from Drone)
- Forge-agnostic (GitHub / Gitea / Forgejo / GitLab)
- Lighter than Concourse (single binary + agents)
- Strong + simple UI

**Weaknesses:**
- Smaller ecosystem than mature platforms
- Migration cost similar to Drone

**Cost**: free OSS; self-host infra.

### Forgejo Actions / Gitea Actions (LOW migration cost; GHA-syntax-compatible)

[Forgejo Actions](https://forgejo.org/docs/latest/user/actions/) (or Gitea Actions) — GHA-syntax-compatible runner that runs `.github/workflows/*.yml` files unchanged. Runs on Forgejo/Gitea repository hosts.

**Strengths:**
- **Workflow files run UNCHANGED** — same YAML, same actions/ ecosystem
- Migration: drop GitHub remote, add Forgejo/Gitea remote; same `.github/workflows/`
- Forgejo is OSS Gitea-fork; community-driven; active
- Self-hosted entirely (forge + actions runner)

**Weaknesses:**
- Requires migrating BOTH source-host (from GitHub) AND CI (from GHA) → bigger product-change
- Gitea/Forgejo SaaS exists (codeberg.org) but not as integrated as GitHub.com
- Lose GitHub-specific affordances (PR-comment-bots / Dependabot / etc) unless replaced

**Cost**: free OSS; self-host infra (Forgejo + runner).

### CircleCI (LOW migration cost; SaaS-tier-bound)

[CircleCI](https://circleci.com/) — SaaS + self-hosted runner option. YAML config.

**Strengths:**
- Mature platform; well-documented
- Good Linux/macOS/Windows support
- Self-hosted runner option for cost-bypass
- Native Rust + Node + Playwright support

**Weaknesses:**
- Free tier (6000 build minutes / month) probably insufficient for bpmux velocity
- Will hit same quota-class problem as GHA; just different vendor
- Migration cost: rewrite `.github/workflows/` → `.circleci/config.yml`

**Cost**: free 6000 min/month (likely insufficient); paid plans scale per concurrent runs.

### Jenkins (HIGH migration cost; mature; OSS)

[Jenkins](https://www.jenkins.io/) — OSS; self-hosted; mature; plugin-driven.

**Strengths:**
- Most-mature OSS CI; plugin ecosystem covers everything
- Declarative + scripted (Groovy DSL) pipeline syntax

**Weaknesses:**
- Heavyweight; substantial operational overhead
- Steep learning curve; Groovy DSL idiosyncratic
- Migration cost HIGH (rewrite from declarative GHA → Jenkinsfile)

**Cost**: free OSS; self-host infra.

## Cost comparison

| Platform | Migration cost | Operational cost | Software cost | Bypasses quota? |
|---|---|---|---|---|
| Self-hosted GHA runner | **ZERO** | $0-20/month (self-host box) | $0 | Yes |
| Forgejo Actions | LOW (workflow unchanged but source-host migration) | $5-20/month (Forgejo + runner) | $0 | Yes (different host) |
| Buildkite | LOW-MEDIUM | $0-20/month (3-agent free) | $0 (free tier) | Yes |
| Drone | MEDIUM | $5-20/month | $0 | Yes |
| Woodpecker | MEDIUM | $5-20/month | $0 | Yes |
| CircleCI | LOW (config-rewrite) | varies | $0-50+/month | Partial (free 6000 min may insufficient) |
| Concourse-CI | HIGH | $5-20/month + ops complexity | $0 | Yes |
| Jenkins | HIGH | $5-20/month + plugin-mgmt | $0 | Yes |

## Developer-experience axis (UX)

**For bpmux's team (1 owner + 3 AI agents)**, dev-experience-axes that matter:

1. **Familiarity → low migration friction**: GHA-syntax-compatible (self-hosted runner / Forgejo Actions) wins; team knows GHA semantics already
2. **Failure-trace observability**: today's `mcp__bpmux-bridge__get_ci_log_for_run` MCP-tool wraps gh CLI; Concourse / Drone / Woodpecker would need new bridge-adapter
3. **Rerun-on-flake**: today's `mcp__bpmux-bridge__rerun_ci` (#1222) wraps GH API; would need new adapter for non-GHA platforms
4. **Pull-vs-push trigger model**: GHA + Drone + Buildkite are push-driven (familiar); Concourse is pull-driven (paradigm shift)
5. **Web UI quality**: Buildkite + GHA + Concourse all good; Jenkins notoriously dated
6. **Operational simplicity**: self-hosted runner < Buildkite < Drone/Woodpecker < Concourse < Jenkins

## Substrate-fit appendix

*Per PM 19:05 UTC executor-naming, dev contributes substrate-fit-appendix at output-time. Open ask:*

@dev — please contribute on:
- **Bridge adapter migration cost** for each candidate (today's `get_ci_state` + `get_ci_log_for_run` + `rerun_ci` wrap GH API; what's the bridge-adapter-rewrite for each platform?)
- **Concrete pipeline-config rewrite estimate** (current `.github/workflows/ci.yml` is 115 lines; how many lines for each candidate?)
- **xvfb + Playwright browser-install** support per platform (any blockers?)
- **Rust toolchain caching**: `Swatinem/rust-cache@v2` is GHA-specific; what's the equivalent on each candidate?
- **Artifact-passing semantics** between build-bpmux + playwright-shard jobs (cross-job binary upload/download)

## Recommendation

**Recommend self-hosted GitHub Actions runner as Primary path** with **Forgejo Actions as Backup-evaluation path** if Ryan wants to migrate AWAY from GitHub entirely.

**Reasoning:**
- **Lowest migration cost**: workflow files run unchanged (zero refactor)
- **Lowest learning cost**: team knows GHA semantics; bridge MCP-tools (#1222 rerun_ci + get_ci_state + get_ci_log_for_run) work without rewrite
- **Bypasses quota**: self-hosted minutes don't count against free tier
- **Setup is single-command** (`./config.sh` on Mac/Linux box)
- **Sister-precedent**: matches today's substrate-fix-by-restricting-surface principle (don't migrate substrate; keep substrate, change runner-environment)

**Rejected: Concourse-CI** for bpmux's case:
- Migration cost HIGH (paradigm shift from push to pull-driven; bridge MCP-tools all need rewrite; 115-line workflow → multi-pipeline rewrite)
- Operational cost HIGH (PostgreSQL + Web + Worker components)
- Concourse shines at multi-team / multi-pipeline use-cases; bpmux is one team / one pipeline

**Forgejo Actions if migrating GitHub itself**: if cost-axis is GitHub-broader (not just Actions), Forgejo with Actions runner is the equivalent-shape escape; same config but on owned-infrastructure.

## Open questions for Ryan (Epic-shape decision)

1. **Migration scope**: just CI runtime (self-hosted GHA runner; keep GitHub repo) OR full GitHub-escape (Forgejo + Forgejo Actions; new repo host)?
2. **Self-host infrastructure**: dedicated box (NUC / Mac mini / VPS) OR repurpose existing dev machine?
3. **Concourse preference revisit**: original ask was Concourse-specific; this research shows higher migration cost than alternatives. Still want Concourse, OR open to self-hosted-GHA-runner / Forgejo-Actions?
4. **Multi-platform support need**: today CI is Linux-only via xvfb; if future need is macOS or Windows for native Electron tests, that affects platform choice (Buildkite / GHA both support multi-OS; Drone/Woodpecker Linux-only)
5. **Maintenance willingness**: self-hosting introduces ops-burden (box uptime / runner upgrades / security). What's acceptable?

## References

- Concourse: https://concourse-ci.org/
- Buildkite: https://buildkite.com/
- Drone: https://www.drone.io/
- Woodpecker: https://woodpecker-ci.org/
- Forgejo Actions: https://forgejo.org/docs/latest/user/actions/
- CircleCI: https://circleci.com/
- Jenkins: https://www.jenkins.io/
- Self-hosted GHA runners: https://docs.github.com/en/actions/hosting-your-own-runners
- bpmux today's CI: `applications/.../.github/workflows/ci.yml`
- bpmux MCP CI tools: `applications/bpmux/src/bridge/tools/get_ci_state.rs` + `get_ci_log_for_run.rs` + `rerun_ci.rs`

## Cross-references

- Sister to today's `feedback_external_substrate_cost_exhaustion` (dev's lesson at 18:37 UTC) — substrate-cost as planning-axis
- Sister to today's `feedback_use_available_tools_not_curl` — bridge MCP-tool layer is canonical query/mutation surface; CI-platform-change must preserve this contract
- Sister to today's `feedback_no_gh_at_all_not_just_mutations` — wherever CI lives, bridge tool layer abstracts platform-specifics
