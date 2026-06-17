import { useEffect, useState } from 'react'
import type { ConnectionStatus } from '../transport/types'
import { RENDERER_IDENTITY } from '../transport/useWebSocket'
import { subscribeWsPtyExit } from '../transport/wsService'
import {
  readTerminalLayout,
  restoreTerminalLayout,
  writeTerminalLayout
} from '../lib/terminalLayout'
import { readTerminalNames, writeTerminalNames } from '../lib/terminalNames'
import {
  closeTerminalRequest,
  listTerminalsRequest,
  spawnTerminalRequest
} from '../lib/terminalApi'

export type TerminalState = 'spawning' | 'ready' | 'spawn-failed' | 'exited'

export interface TerminalEntry {
  key: string
  state: TerminalState
  id?: string
  cwd?: string
  error?: string
}

export interface ConfirmClose {
  key: string
  id: string
}

export interface UseTerminalsResult {
  terminals: TerminalEntry[]
  activeKey: string | null
  terminalNames: Record<string, string>
  confirmClose: ConfirmClose | null
  spawnTerminal: () => void
  restartTerminal: (key: string) => void
  selectTerminal: (key: string) => void
  renameTerminal: (id: string, name: string) => void
  closeTerminal: (key: string) => void
  confirmCloseTerminal: () => void
  clearConfirmClose: () => void
  clearActiveTerminal: () => void
}

export const useTerminals = (
  status: ConnectionStatus,
  bridge: Window['bpmux']
): UseTerminalsResult => {
  const [terminals, setTerminals] = useState<TerminalEntry[]>([])
  const [activeKey, setActiveKey] = useState<string | null>(null)
  const [restored, setRestored] = useState<boolean>(false)
  const [terminalNames, setTerminalNames] = useState<Record<string, string>>(() =>
    readTerminalNames(RENDERER_IDENTITY)
  )
  const [confirmClose, setConfirmClose] = useState<ConfirmClose | null>(null)

  const requestSpawn = (key: string): void => {
    if (!bridge) return
    const markFailed = (error: string): void =>
      setTerminals((prev) =>
        prev.map((t) => (t.key === key ? { ...t, state: 'spawn-failed', error } : t))
      )
    void spawnTerminalRequest(bridge)
      .then(async (res) => {
        if (!res.ok) {
          const detail = await res.text().catch(() => '')
          markFailed(detail.length > 0 ? detail : `spawn failed (${res.status})`)
          return
        }
        const body = (await res.json()) as { terminal_id?: string; cwd?: string }
        if (!body.terminal_id) {
          markFailed('spawn response missing terminal_id')
          return
        }
        const id = body.terminal_id
        setTerminals((prev) => {
          if (!prev.some((t) => t.key === key)) {
            void closeTerminalRequest(bridge, id, true).catch((err: unknown) =>
              console.error('bpmux: orphaned terminal reap failed', err)
            )
            return prev
          }
          return prev.map((t) =>
            t.key === key ? { ...t, state: 'ready', id, cwd: body.cwd ?? '' } : t
          )
        })
      })
      .catch((e: unknown) => markFailed(String(e)))
  }

  const spawnTerminal = (): void => {
    if (!bridge) return
    const key = crypto.randomUUID()
    setTerminals((prev) => [...prev, { key, state: 'spawning' }])
    setActiveKey(key)
    requestSpawn(key)
  }

  const restartTerminal = (key: string): void => {
    if (!bridge) return
    setTerminals((prev) => prev.map((t) => (t.key === key ? { key, state: 'spawning' } : t)))
    setActiveKey(key)
    requestSpawn(key)
  }

  const selectTerminal = (key: string): void => {
    setActiveKey(key)
  }

  const renameTerminal = (id: string, name: string): void => {
    setTerminalNames((prev) => {
      const next = { ...prev }
      const trimmed = name.trim()
      if (trimmed.length > 0) {
        next[id] = trimmed
      } else {
        delete next[id]
      }
      writeTerminalNames(RENDERER_IDENTITY, next)
      return next
    })
  }

  const removePane = (key: string): void => {
    const remaining = terminals.filter((t) => t.key !== key)
    setTerminals(remaining)
    if (activeKey === key) {
      setActiveKey(remaining.length > 0 ? remaining[remaining.length - 1].key : null)
    }
  }

  const closeTerminal = (key: string): void => {
    const closing = terminals.find((t) => t.key === key)
    if (!closing?.id || !bridge) {
      removePane(key)
      return
    }
    const id = closing.id
    void closeTerminalRequest(bridge, id, false)
      .then((res) => {
        if (res.status === 409) {
          setConfirmClose({ key, id })
          return
        }
        removePane(key)
      })
      .catch((e: unknown) => {
        console.error('bpmux: terminal close failed', e)
        removePane(key)
      })
  }

  const confirmCloseTerminal = (): void => {
    if (!confirmClose) return
    const { key, id } = confirmClose
    setConfirmClose(null)
    void closeTerminalRequest(bridge, id, true).catch((e: unknown) =>
      console.error('bpmux: terminal force-close failed', e)
    )
    removePane(key)
  }

  useEffect(() => {
    return subscribeWsPtyExit((terminalId) => {
      setTerminals((prev) => prev.map((t) => (t.id === terminalId ? { ...t, state: 'exited' } : t)))
    })
  }, [])

  useEffect(() => {
    if (restored || status !== 'connected' || !bridge) return
    let cancelled = false
    const restoreTerminals = async (): Promise<void> => {
      const maxAttempts = 4
      for (let attempt = 1; attempt <= maxAttempts; attempt += 1) {
        try {
          const res = await listTerminalsRequest(bridge)
          if (cancelled) return
          if (!res.ok) throw new Error(`terminals list failed (${res.status})`)
          const body = (await res.json()) as {
            terminals?: { terminal_id: string; cwd?: string }[]
          }
          if (cancelled) return
          const live = body.terminals ?? []
          const cwdById = new Map(live.map((t) => [t.terminal_id, t.cwd ?? '']))
          const { orderedIds, activeId } = restoreTerminalLayout(
            live.map((t) => t.terminal_id),
            readTerminalLayout(RENDERER_IDENTITY)
          )
          const entries: TerminalEntry[] = orderedIds.map((id) => ({
            key: crypto.randomUUID(),
            id,
            state: 'ready',
            cwd: cwdById.get(id) ?? ''
          }))
          if (entries.length > 0) {
            setTerminals(entries)
            setActiveKey(entries.find((e) => e.id === activeId)?.key ?? null)
          }
          setRestored(true)
          return
        } catch (e: unknown) {
          if (cancelled) return
          if (attempt === maxAttempts) {
            console.error('bpmux: terminal restore failed', e)
            return
          }
          await new Promise((resolve) => setTimeout(resolve, 150 * 2 ** (attempt - 1)))
        }
      }
    }
    void restoreTerminals()
    return () => {
      cancelled = true
    }
  }, [status, restored, bridge])

  useEffect(() => {
    if (!restored) return
    const order = terminals.filter((t) => t.id).map((t) => t.id as string)
    const active = terminals.find((t) => t.key === activeKey)
    writeTerminalLayout(RENDERER_IDENTITY, { order, activeId: active?.id ?? null })
  }, [terminals, activeKey, restored])

  return {
    terminals,
    activeKey,
    terminalNames,
    confirmClose,
    spawnTerminal,
    restartTerminal,
    selectTerminal,
    renameTerminal,
    closeTerminal,
    confirmCloseTerminal,
    clearConfirmClose: () => setConfirmClose(null),
    clearActiveTerminal: () => setActiveKey(null)
  }
}
