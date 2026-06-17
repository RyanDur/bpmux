import { useState } from 'react'
import { useMessageFilter, type MessageFilter } from './useMessageFilter'
import { readScope, writeScope } from '../lib/scopeStorage'

export interface UseScopeResult {
  recipient: string[] | null
  allView: boolean
  filter: MessageFilter
  selectRecipient: (r: string[] | null) => void
  selectAllView: () => void
}

export const useScope = (onScopeChange: () => void): UseScopeResult => {
  const initialScope = readScope()
  const [recipient, setRecipient] = useState<string[] | null>(
    initialScope?.kind === 'scoped' ? initialScope.value : null
  )
  const [allView, setAllView] = useState<boolean>(initialScope?.kind !== 'scoped')
  const channelKey = allView
    ? 'all'
    : recipient && recipient.length > 0
      ? `scoped:${[...recipient].sort().join(',')}`
      : 'broadcast'
  const filter = useMessageFilter(channelKey)

  const selectRecipient = (r: string[] | null): void => {
    onScopeChange()
    setAllView(false)
    setRecipient(r)
    writeScope(r && r.length > 0 ? { kind: 'scoped', value: r } : { kind: 'broadcast' })
  }

  const selectAllView = (): void => {
    onScopeChange()
    setAllView(true)
    setRecipient(null)
    writeScope({ kind: 'all' })
  }

  return { recipient, allView, filter, selectRecipient, selectAllView }
}
