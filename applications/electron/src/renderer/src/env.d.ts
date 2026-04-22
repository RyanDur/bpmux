/// <reference types="vite/client" />

// `window.bpmux` is contributed by the preload script (owned by the
// parallel main-process agent — see their preload/index.ts). We declare
// the shape here so renderer code can use it without `any` and without
// touching preload files.
declare global {
  interface Window {
    bpmux?: {
      wsUrl: string
      authToken: string
      apiBase: string
    }
  }
}

export {}
