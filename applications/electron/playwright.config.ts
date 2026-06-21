import { defineConfig } from '@playwright/test'

const SERIAL_HEAVY_SPECS = [
  '**/real-backend.spec.ts',
  '**/a11y-audit.spec.ts',
  '**/canary-env-leak.spec.ts',
  '**/survive-refresh-reattach.spec.ts',
  '**/n-terminals-switch.spec.ts',
  '**/terminal-copy.spec.ts',
  '**/confirm-before-close.spec.ts',
  '**/exited-restart-close.spec.ts'
]

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
  },
  projects: [
    { name: 'serial', testMatch: SERIAL_HEAVY_SPECS },
    { name: 'main', testIgnore: SERIAL_HEAVY_SPECS }
  ]
})
