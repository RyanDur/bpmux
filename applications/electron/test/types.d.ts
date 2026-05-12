// Test-layer type augmentations for #1303 RTL+real-backend infrastructure.
// Mirrors the window.bpmux declaration from src/renderer/src/env.d.ts so
// test files (which sit outside src/renderer/src/) get the same type.

declare global {
  interface Window {
    bpmux?: {
      wsUrl: string
      authToken: string
      apiBase: string
      onQuoteComment?: (
        callback: (payload: { commentId: string; author: string; text: string }) => void
      ) => () => void
    }
  }
}

// Augment vitest's ProvidedContext so inject() is type-safe for the
// keys our globalSetup provides (bpmuxBackendPort, bpmuxAuthToken).
declare module 'vitest' {
  export interface ProvidedContext {
    bpmuxBackendPort: number
    bpmuxAuthToken: string
  }
}

export {}
