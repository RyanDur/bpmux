declare global {
  interface Window {
    bpmux: {
      wsUrl: string
      authToken: string
      apiBase: string
      e2e: boolean
      onQuoteComment?: (
        callback: (payload: { commentId: string; author: string; text: string }) => void
      ) => () => void
    }
    __bpmuxTermText?: Record<string, () => string>
  }
}
