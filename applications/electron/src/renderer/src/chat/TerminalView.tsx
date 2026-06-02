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
  terminalId: string
  isActive: boolean
}

const readToken = (name: string, fallback: string): string => {
  const value = getComputedStyle(document.body).getPropertyValue(name).trim()
  return value.length > 0 ? value : fallback
}

export const TerminalView = ({ terminalId, isActive }: TerminalViewProps): React.JSX.Element => {
  const containerRef = useRef<HTMLDivElement>(null)
  const termRef = useRef<Terminal | null>(null)

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
    termRef.current = term
    const fit = new FitAddon()
    term.loadAddon(fit)
    term.open(container)
    fit.fit()
    term.focus()

    const unsubscribe = subscribeWsPtyOutput(terminalId, (data) => {
      term.write(decodePtyData(data))
    })
    term.onData((keystroke) => sendPtyInput(terminalId, keystroke))
    sendPtySubscribe(terminalId)

    const onResize = (): void => fit.fit()
    window.addEventListener('resize', onResize)

    return () => {
      window.removeEventListener('resize', onResize)
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
      ref={containerRef}
      className="terminal-view"
      data-test={`terminal-${terminalId}`}
      aria-label={`terminal ${terminalId}`}
    />
  )
}
