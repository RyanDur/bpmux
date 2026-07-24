import { useEffect, useState } from 'react'

const DRAFT_STORAGE_PREFIX = 'bpmux-draft:'

const readDraft = (channelKey: string): string => {
  if (typeof window === 'undefined') return ''
  try {
    return window.localStorage.getItem(`${DRAFT_STORAGE_PREFIX}${channelKey}`) ?? ''
  } catch {
    return ''
  }
}

const writeDraft = (channelKey: string, value: string): void => {
  if (typeof window === 'undefined') return
  try {
    if (value.length === 0) {
      window.localStorage.removeItem(`${DRAFT_STORAGE_PREFIX}${channelKey}`)
    } else {
      window.localStorage.setItem(`${DRAFT_STORAGE_PREFIX}${channelKey}`, value)
    }
  } catch {
    return
  }
}

export type DraftStorage = {
  value: string
  setValue: (value: string) => void
}

export const useDraftStorage = (channelKey: string): DraftStorage => {
  const [value, setValue] = useState(() => readDraft(channelKey))
  useEffect(() => {
    writeDraft(channelKey, value)
  }, [channelKey, value])
  return { value, setValue }
}
