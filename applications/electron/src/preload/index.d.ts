declare global {
  interface Window {
    bpmux: {
      wsUrl: string
      authToken: string
      apiBase: string
    }
  }
}
