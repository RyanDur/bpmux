declare global {
  interface Window {
    bpmux: {
      wsUrl: string
      authToken: string
      apiBase: string
      onQuoteComment?: (
        callback: (payload: { commentId: string; author: string; text: string }) => void
      ) => () => void
    }
  }
}
