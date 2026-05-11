import { defineConfig } from '@playwright/test'

export default defineConfig({
  // Each test launches its own isolated Electron+bpmux pair with a
  // unique BPMUX_BACKEND_PORT (getFreePort), unique session-base +
  // user-data-dir + auth-token-path (all mkdtempSync), so tests are
  // safe to parallelize. The earlier `workers: 1` was historical —
  // port-and-state isolation was already in place.
  fullyParallel: true,
  workers: 2,
  retries: 1,
  testDir: 'tests',
  // #1252: bumped testTimeout from Playwright default 30s to 90s.
  // Self-hosted macOS arm64 runs the renderer through the real
  // WindowServer (vs Linux+xvfb virtual framebuffer in GHA-hosted
  // CI); per-test wall-clock is ~3× slower for Electron-window-
  // heavy specs. Overnight CI on f9ebe47 showed stable tests
  // (desk-check-357, desk-check-361, etc.) timing out at 30s; the
  // bump trades a wider ceiling for empirical passability without
  // hiding genuine regressions (still well below the 2-min mark
  // where a test indicates a real hang).
  timeout: 90_000,
  // Bump default action / expect-poll timeouts for parallel runs
  // where multiple Electron+bpmux backends share CPU at warmup.
  // Renderer derived state (e.g., sidebar-direct-{role} from
  // message-history) can take a couple seconds longer to settle
  // under contention than the 5s default.
  expect: {
    timeout: 10_000
  },
  use: {
    actionTimeout: 10_000
  }
})
