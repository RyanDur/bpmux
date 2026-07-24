import { resolve } from 'path'
import { defineConfig } from 'electron-vite'
import react from '@vitejs/plugin-react'

export default defineConfig({
  main: {},
  preload: {},
  renderer: {
    resolve: {
      alias: {
        '@renderer': resolve('src/renderer/src'),
        '@components': resolve('src/renderer/src/components'),
        '@state': resolve('src/renderer/src/state'),
        '@transport': resolve('src/renderer/src/transport'),
        '@test-support': resolve('src/renderer/src/test-support')
      }
    },
    plugins: [react()]
  }
})
