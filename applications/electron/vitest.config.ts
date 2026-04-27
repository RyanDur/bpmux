import { defineConfig } from 'vitest/config'
import { resolve } from 'path'

export default defineConfig({
  test: {
    environment: 'jsdom',
    include: ['src/**/*.test.{ts,tsx}'],
    exclude: ['node_modules', 'tests', 'out']
  },
  resolve: {
    alias: {
      '@renderer': resolve(__dirname, './src/renderer/src')
    }
  }
})
