import { useCallback, useState } from 'react'

const MAX_HISTORY = 50

export type HistoryNavigation = {
  record: (text: string) => void
  prev: (currentValue: string) => string | null
  next: () => string | null
  reset: () => void
  findMatch: (query: string, fromIndex?: number) => number | null
  valueAt: (index: number) => string | undefined
  commitIndex: (index: number) => void
  size: number
}

export const useInputHistory = (): HistoryNavigation => {
  const [history, setHistory] = useState<string[]>([])
  const [index, setIndex] = useState<number | null>(null)
  const [draft, setDraft] = useState<string>('')

  const record = useCallback((text: string): void => {
    setHistory((prev) => {
      if (prev[prev.length - 1] === text) return prev
      const next = [...prev, text]
      if (next.length > MAX_HISTORY) next.shift()
      return next
    })
    setIndex(null)
    setDraft('')
  }, [])

  const prev = useCallback(
    (currentValue: string): string | null => {
      if (history.length === 0) return null
      let nextIdx: number
      if (index === null) {
        setDraft(currentValue)
        nextIdx = history.length - 1
      } else if (index > 0) {
        nextIdx = index - 1
      } else {
        return null
      }
      setIndex(nextIdx)
      return history[nextIdx]
    },
    [history, index]
  )

  const next = useCallback((): string | null => {
    if (index === null) return null
    if (index < history.length - 1) {
      const nextIdx = index + 1
      setIndex(nextIdx)
      return history[nextIdx]
    }
    setIndex(null)
    return draft
  }, [history, index, draft])

  const reset = useCallback((): void => {
    setIndex(null)
    setDraft('')
  }, [])

  const findMatch = useCallback(
    (query: string, fromIndex?: number): number | null => {
      if (!query) return null
      const start = fromIndex ?? history.length - 1
      for (let i = start; i >= 0; i -= 1) {
        if (history[i].includes(query)) return i
      }
      return null
    },
    [history]
  )

  const valueAt = useCallback((i: number): string | undefined => history[i], [history])

  const commitIndex = useCallback((i: number): void => {
    setIndex(i)
  }, [])

  return {
    record,
    prev,
    next,
    reset,
    findMatch,
    valueAt,
    commitIndex,
    size: history.length
  }
}
