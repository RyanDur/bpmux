import { useEffect, useRef, useState } from 'react'
import { Terminal } from '@xterm/xterm'
import { FitAddon } from '@xterm/addon-fit'
import '@xterm/xterm/css/xterm.css'
import {
  decodePtyData,
  sendPtyInput,
  sendPtyResize,
  sendPtySubscribe,
  subscribeWsPtyOutput
} from '../transport/wsService'
import './TerminalView.css'

interface TerminalViewProps {
  terminalId: string
  cwd: string
  isActive: boolean
}

const TERM_NAME = 'xterm-256color'

const readToken = (name: string, fallback: string): string => {
  const value = getComputedStyle(document.body).getPropertyValue(name).trim()
  return value.length > 0 ? value : fallback
}

export const TerminalView = ({
  terminalId,
  cwd,
  isActive
}: TerminalViewProps): React.JSX.Element => {
  const screenRef = useRef<HTMLElement>(null)
  const termRef = useRef<Terminal | null>(null)
  const [dims, setDims] = useState<{ cols: number; rows: number }>({ cols: 0, rows: 0 })

  useEffect(() => {
    const screen = screenRef.current
    if (!screen) return undefined

    const term = new Terminal({
      cursorBlink: true,
      convertEol: false,
      fontFamily: readToken('--font-mono', 'ui-monospace, SFMono-Regular, Menlo, monospace'),
      theme: {
        background: readToken('--bg-base', '#1e1e1e'),
        foreground: readToken('--fg-base', '#d4d4d4')
      }
    })
    termRef.current = term
    const fit = new FitAddon()
    term.loadAddon(fit)
    term.open(screen)

    const syncSize = (): void => {
      fit.fit()
      setDims({ cols: term.cols, rows: term.rows })
      sendPtyResize(terminalId, term.cols, term.rows)
    }
    syncSize()
    term.focus()

    const unsubscribe = subscribeWsPtyOutput(terminalId, (data) => {
      term.write(decodePtyData(data))
    })
    term.onData((keystroke) => sendPtyInput(terminalId, keystroke))
    sendPtySubscribe(terminalId)

    window.addEventListener('resize', syncSize)

    return () => {
      window.removeEventListener('resize', syncSize)
      unsubscribe()
      term.dispose()
      termRef.current = null
    }
  }, [terminalId])

  useEffect(() => {
    if (!isActive) return undefined
    const raf = requestAnimationFrame(() => termRef.current?.focus())
    return () => cancelAnimationFrame(raf)
  }, [isActive])

  return (
    <section
      className="terminal-view"
      data-test={`terminal-${terminalId}`}
      aria-label={`terminal ${terminalId}`}
    >
      <header className="terminal-header" data-test="terminal-header">
        {cwd} · {dims.cols}×{dims.rows} · {TERM_NAME}
      </header>
      <section ref={screenRef} className="terminal-screen" aria-label="terminal output" />
    </section>
  )
}
