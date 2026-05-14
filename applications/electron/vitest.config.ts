import { defineConfig } from 'vitest/config'
import { resolve } from 'path'

// #1303 Epic #1247 slice 1: RTL+real-backend infrastructure.
// globalSetup spawns ONE bpmux backend subprocess for the test-run
// (MemoryStorage via BPMUX_STORAGE_BACKEND=memory + BPMUX_TEST_MODE=1);
// setupFiles polyfills WebSocket in jsdom so renderer code can connect.
// Pure renderer unit tests (the 227 existing ones) tolerate the spawn
// cost (~1-2s amortized across the full run). Opt-in pattern: tests
// import `renderApp` from `test/fixtures/renderApp.tsx`; tests that
// don't need the backend ignore globalSetup's provided keys.
export default defineConfig({
  test: {
    environment: 'jsdom',
    include: ['src/**/*.test.{ts,tsx}'],
    exclude: ['node_modules', 'tests', 'out'],
    globalSetup: './test/setup/globalSetup.ts',
    setupFiles: ['./test/setup/setupFile.ts'],
    // #1306 slice 4.1: tests share the per-run bpmux backend (one
    // process spawned by globalSetup). Parallel workers race against
    // its message-store via reset+seed cycles; serializing tests
    // avoids cross-worker bleed. The substrate-cost is acceptable
    // because each test still completes in <200ms and the backend
    // spawn-cost is amortized once.
    pool: 'threads',
    poolOptions: { threads: { singleThread: true } },
    fileParallelism: false
  },
  resolve: {
    alias: {
      '@renderer': resolve(__dirname, './src/renderer/src')
    }
  }
})
