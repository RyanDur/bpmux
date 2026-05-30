import { useEffect, useRef } from 'react'
import { Terminal } from '@xterm/xterm'
import { FitAddon } from '@xterm/addon-fit'
import '@xterm/xterm/css/xterm.css'
import {
  decodePtyData,
  sendPtyInput,
  sendPtySubscribe,
  subscribeWsPtyOutput
} from '../transport/wsService'
import './TerminalView.css'

interface TerminalViewProps {
  role: string
}

const readToken = (name: string, fallback: string): string => {
  const value = getComputedStyle(document.body).getPropertyValue(name).trim()
  return value.length > 0 ? value : fallback
}

const ensureRoomSpawned = async (role: string): Promise<void> => {
  const bridge = window.bpmux
  if (!bridge) return
  await fetch(`${bridge.apiBase}/api/rooms/${encodeURIComponent(role)}/spawn`, {
    method: 'POST',
    headers: { authorization: `Bearer ${bridge.authToken}` }
  })
}

export const TerminalView = ({ role }: TerminalViewProps): React.JSX.Element => {
  const containerRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    const container = containerRef.current
    if (!container) return undefined

    const term = new Terminal({
      cursorBlink: true,
      convertEol: false,
      fontFamily: readToken('--font-mono', 'ui-monospace, SFMono-Regular, Menlo, monospace'),
      theme: {
        background: readToken('--bg-base', '#1e1e1e'),
        foreground: readToken('--fg-base', '#d4d4d4')
      }
    })
    const fit = new FitAddon()
    term.loadAddon(fit)
    term.open(container)
    fit.fit()

    const unsubscribe = subscribeWsPtyOutput(role, (data) => {
      term.write(decodePtyData(data))
    })
    term.onData((keystroke) => sendPtyInput(role, keystroke))
    void ensureRoomSpawned(role).then(() => sendPtySubscribe(role))

    const onResize = (): void => fit.fit()
    window.addEventListener('resize', onResize)

    return () => {
      window.removeEventListener('resize', onResize)
      unsubscribe()
      term.dispose()
    }
  }, [role])

  return (
    <section
      ref={containerRef}
      className="terminal-view"
      data-test={`terminal-${role}`}
      aria-label={`${role} terminal`}
    />
  )
}
