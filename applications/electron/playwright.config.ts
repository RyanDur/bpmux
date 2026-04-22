import { defineConfig } from '@playwright/test'

export default defineConfig({
  workers: 1, // Electron instances can't parallel on the same port.
  testDir: 'tests',
})
