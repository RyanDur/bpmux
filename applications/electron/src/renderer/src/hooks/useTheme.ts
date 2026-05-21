import { useCallback, useEffect, useState } from 'react'

export type Theme = 'dark' | 'light'

const STORAGE_KEY = 'bpmux-theme'

const resolveInitial = (): Theme => {
  if (typeof window === 'undefined') return 'dark'
  const stored = window.localStorage.getItem(STORAGE_KEY)
  if (stored === 'light' || stored === 'dark') return stored
  if (typeof window.matchMedia === 'function') {
    const prefersLight = window.matchMedia('(prefers-color-scheme: light)').matches
    return prefersLight ? 'light' : 'dark'
  }
  return 'dark'
}

export const useTheme = (): { theme: Theme; toggle: () => void; set: (t: Theme) => void } => {
  const [theme, setTheme] = useState<Theme>(resolveInitial)

  useEffect(() => {
    if (typeof document !== 'undefined') {
      document.documentElement.dataset.theme = theme
    }
    try {
      window.localStorage.setItem(STORAGE_KEY, theme)
    } catch {
      // storage can fail silently (private mode, etc.)
    }
  }, [theme])

  const toggle = useCallback((): void => {
    setTheme((prev) => (prev === 'dark' ? 'light' : 'dark'))
  }, [])

  const set = useCallback((t: Theme): void => {
    setTheme(t)
  }, [])

  return { theme, toggle, set }
}
