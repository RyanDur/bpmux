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
      // #371: main-process-fired IPC for "Quote in reply" context-menu
      // clicks. DrillIn subscribes on mount; unsubscribe on unmount.
      // Optional — renderers running outside Electron (unit tests,
      // future web surface) omit the handler and fall back to the
      // Cmd+E keyboard path from #366.
      onQuoteComment?: (
        callback: (payload: {
          commentId: string
          author: string
          text: string
        }) => void
      ) => () => void
    }
  }
}

export {}
