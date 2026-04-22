import { useCallback, useEffect, useState } from 'react'

// #369: chat density preference. Two modes — compact (tighter
// spacing, smaller avatar) and comfortable (current / default
// spacing). Sets `data-density` on <html> so CSS can branch off the
// root selector; JS stays minimal (state + localStorage persistence
// + attribute write), all presentation differences live in CSS.

export type Density = 'compact' | 'comfortable'

const STORAGE_KEY = 'bpmux-density'

function resolveInitial(): Density {
  if (typeof window === 'undefined') return 'comfortable'
  const stored = window.localStorage.getItem(STORAGE_KEY)
  if (stored === 'compact' || stored === 'comfortable') return stored
  return 'comfortable'
}

export function useDensity(): {
  density: Density
  toggle: () => void
  set: (d: Density) => void
} {
  const [density, setDensity] = useState<Density>(resolveInitial)

  useEffect(() => {
    if (typeof document !== 'undefined') {
      document.documentElement.dataset.density = density
    }
    try {
      window.localStorage.setItem(STORAGE_KEY, density)
    } catch {
      // storage can fail silently (private mode, etc.) — density
      // simply won't persist this session. Non-load-bearing on
      // correctness.
    }
  }, [density])

  const toggle = useCallback((): void => {
    setDensity((prev) => (prev === 'compact' ? 'comfortable' : 'compact'))
  }, [])

  const set = useCallback((d: Density): void => {
    setDensity(d)
  }, [])

  return { density, toggle, set }
}
