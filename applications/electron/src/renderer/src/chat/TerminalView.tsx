import { useEffect, useRef, useState } from 'react'
import { Terminal } from '@xterm/xterm'
import { FitAddon } from '@xterm/addon-fit'
import '@xterm/xterm/css/xterm.css'
import {
  decodePtyData,
  sendPtyInput,
  sendPtyResize,
  sendPtySubscribe,
  subscribePtyMode,
  subscribePtyOutput
} from '../transport/session'
import { composeTerminalFontFamily } from '../lib/terminalFont'
import { TerminalCompose } from './TerminalCompose'
import './TerminalView.css'

interface TerminalViewProps {
  terminalId: string
  cwd: string
  isActive: boolean
  onClose?: () => void
}

const TERM_NAME = 'xterm-256color'

const readToken = (name: string, fallback: string): string => {
  const value = getComputedStyle(document.body).getPropertyValue(name).trim()
  return value.length > 0 ? value : fallback
}

const cwdFromOsc7 = (payload: string): string | null => {
  try {
    const { pathname } = new URL(payload)
    return pathname.length > 0 ? decodeURIComponent(pathname) : null
  } catch {
    return null
  }
}

const copyOnCmdC = (term: Terminal, event: KeyboardEvent): boolean => {
  if (event.type === 'keydown' && event.metaKey && !event.ctrlKey && event.key === 'c') {
    const selection = term.getSelection()
    if (selection.length > 0) {
      void navigator.clipboard.writeText(selection)
      return false
    }
  }
  return true
}

export const TerminalView = ({
  terminalId,
  cwd,
  isActive,
  onClose
}: TerminalViewProps): React.JSX.Element => {
  const screenRef = useRef<HTMLElement>(null)
  const termRef = useRef<Terminal | null>(null)
  const composeRef = useRef<HTMLTextAreaElement>(null)
  const [dims, setDims] = useState<{ cols: number; rows: number }>({ cols: 0, rows: 0 })
  const [liveCwd, setLiveCwd] = useState(cwd)
  const [bracketedPaste, setBracketedPaste] = useState(false)

  useEffect(() => {
    const screen = screenRef.current
    if (!screen) return undefined

    const term = new Terminal({
      cursorBlink: true,
      convertEol: false,
      fontFamily: composeTerminalFontFamily(readToken('--font-mono', '')),
      theme: {
        background: readToken('--bg-base', '#1e1e1e'),
        foreground: readToken('--fg-base', '#d4d4d4')
      }
    })
    termRef.current = term
    const fit = new FitAddon()
    term.loadAddon(fit)
    term.open(screen)
    term.attachCustomKeyEventHandler((event) => copyOnCmdC(term, event))

    const syncSize = (): void => {
      fit.fit()
      setDims({ cols: term.cols, rows: term.rows })
      sendPtyResize(terminalId, term.cols, term.rows)
    }
    syncSize()
    term.focus()

    const unsubscribe = subscribePtyOutput(terminalId, (data) => {
      term.write(decodePtyData(data))
    })
    const unsubscribeMode = subscribePtyMode(terminalId, setBracketedPaste)
    const osc7 = term.parser.registerOscHandler(7, (payload) => {
      const next = cwdFromOsc7(payload)
      if (next) setLiveCwd(next)
      return true
    })
    term.onData((keystroke) => sendPtyInput(terminalId, keystroke))
    sendPtySubscribe(terminalId)

    window.addEventListener('resize', syncSize)

    return () => {
      window.removeEventListener('resize', syncSize)
      unsubscribe()
      unsubscribeMode()
      osc7.dispose()
      term.dispose()
      termRef.current = null
    }
  }, [terminalId])

  useEffect(() => {
    const term = termRef.current
    if (term) term.options.disableStdin = !bracketedPaste
    if (!isActive) return undefined
    const raf = requestAnimationFrame(() => {
      if (bracketedPaste) termRef.current?.focus()
      else composeRef.current?.focus()
    })
    return () => cancelAnimationFrame(raf)
  }, [isActive, bracketedPaste])

  return (
    <section
      className="terminal-view"
      data-test={`terminal-${terminalId}`}
      aria-label={`terminal ${terminalId}`}
    >
      <header className="terminal-header">
        <small
          className="terminal-header-meta"
          aria-label="terminal location and size"
        >{`${liveCwd} · ${dims.cols}×${dims.rows} · ${TERM_NAME}`}</small>
        <button
          type="button"
          className="terminal-header-close"
          aria-label="close terminal"
          title="close terminal"
          onClick={onClose}
        >
          ✕
        </button>
      </header>
      <section
        className="terminal-screen"
        aria-label="terminal output"
        data-input-mode={bracketedPaste ? 'raw' : 'cooked'}
      >
        <section ref={screenRef} className="terminal-screen-canvas" />
      </section>
      {!bracketedPaste && (
        <TerminalCompose
          composeKey={terminalId}
          onSubmit={(line) => sendPtyInput(terminalId, line + '\r')}
          inputRef={composeRef}
        />
      )}
    </section>
  )
}
