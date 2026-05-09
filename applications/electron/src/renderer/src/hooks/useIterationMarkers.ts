import { useEffect, useState } from 'react'

export interface IterationMarker {
  id: number
  theme_name: string
  position: number
  created_at: string
}

interface Args {
  apiBase: string | undefined
  authToken: string | undefined
}

export function useIterationMarkers({ apiBase, authToken }: Args): IterationMarker[] {
  const [markers, setMarkers] = useState<IterationMarker[]>([])

  useEffect(() => {
    if (!apiBase) return
    const controller = new AbortController()
    fetch(`${apiBase}/api/iteration_markers`, {
      headers: { Authorization: `Bearer ${authToken ?? ''}` },
      signal: controller.signal
    })
      .then(async (res) => {
        if (!res.ok) return
        const data = (await res.json()) as IterationMarker[]
        setMarkers(data)
      })
      .catch((err) => {
        if ((err as Error)?.name === 'AbortError') return
      })
    return () => controller.abort()
  }, [apiBase, authToken])

  return markers
}
