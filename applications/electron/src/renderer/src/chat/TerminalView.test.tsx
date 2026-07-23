import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest'
import { cleanup, render, screen, waitFor } from '@testing-library/react'
import type { Terminal } from '@xterm/xterm'
import { TerminalView } from './TerminalView'
import { copyOnCmdC } from './terminalCopy'
import { deliverServerFrameForTest, resetStateForTest, encodeKeystroke } from '../state'

const osc7Sequence = (path: string): string =>
  `${String.fromCharCode(27)}]7;file://${path}${String.fromCharCode(7)}`

const ptyOutputFrame = (terminalId: string, raw: string): string =>
  JSON.stringify({ type: 'pty_output', terminal_id: terminalId, data: encodeKeystroke(raw) })

const ptyModeFrame = (terminalId: string, bracketedPaste: boolean): string =>
  JSON.stringify({ type: 'pty_mode', terminal_id: terminalId, bracketed_paste: bracketedPaste })

describe('TerminalView — the header follows an OSC-7 cwd update from the PTY stream', () => {
  beforeEach(() => resetStateForTest())
  afterEach(() => {
    cleanup()
    resetStateForTest()
  })

  test('an OSC-7 file:// sequence arriving as pty_output updates the header to the new dir', async () => {
    render(<TerminalView terminalId="osc7-unit" cwd="/home/start" isActive={true} />)
    expect(screen.getByText(/\/home\/start/)).toBeTruthy()

    deliverServerFrameForTest(ptyOutputFrame('osc7-unit', osc7Sequence('/new/live/dir')))

    expect(await screen.findByText(/\/new\/live\/dir/)).toBeTruthy()
  })
})

describe('TerminalView — the input-mode mirrors the live bracketed-paste state from pty_mode', () => {
  beforeEach(() => resetStateForTest())
  afterEach(() => {
    cleanup()
    resetStateForTest()
  })

  test('defaults to UNKNOWN until the server signals, then flips raw/cooked on pty_mode', async () => {
    render(<TerminalView terminalId="mode-unit" cwd="/home/start" isActive={true} />)
    const screenRegion = screen.getByLabelText('terminal output')
    expect(screenRegion.getAttribute('data-input-mode')).toBe('unknown')

    deliverServerFrameForTest(ptyModeFrame('mode-unit', true))
    await waitFor(() => expect(screenRegion.getAttribute('data-input-mode')).toBe('raw'))

    deliverServerFrameForTest(ptyModeFrame('mode-unit', false))
    await waitFor(() => expect(screenRegion.getAttribute('data-input-mode')).toBe('cooked'))
  })

  test('the compose-line is absent until a POSITIVE cooked signal — no relocation before the server speaks', async () => {
    render(<TerminalView terminalId="compose-unit" cwd="/home/start" isActive={true} />)
    expect(screen.queryByLabelText('terminal input')).toBeNull()

    deliverServerFrameForTest(ptyModeFrame('compose-unit', false))
    expect(await screen.findByLabelText('terminal input')).toBeTruthy()

    deliverServerFrameForTest(ptyModeFrame('compose-unit', true))
    await waitFor(() => expect(screen.queryByLabelText('terminal input')).toBeNull())
  })
})

describe('TerminalView — a backgrounded (inactive) terminal still receives and buffers its live stream (bpmux keeps streaming the inactive pane)', () => {
  beforeEach(() => {
    resetStateForTest()
    window.bpmux = { e2e: true, wsUrl: '', authToken: '', apiBase: '' }
  })
  afterEach(() => {
    cleanup()
    resetStateForTest()
    delete window.__bpmuxTermText
    delete window.bpmux
  })

  test('pty_output delivered to an isActive=false TerminalView lands in its buffer — the subscription is not gated on active', async () => {
    render(<TerminalView terminalId="bg-pane" cwd="/home/start" isActive={false} />)

    deliverServerFrameForTest(ptyOutputFrame('bg-pane', 'STREAM_WHILE_HIDDEN_42\r\n'))

    await waitFor(() =>
      expect(window.__bpmuxTermText?.['bg-pane']?.() ?? '').toContain('STREAM_WHILE_HIDDEN_42')
    )
  })
})

describe('TerminalView — the Cmd-C copy handler does NOT shadow Ctrl-C (SIGINT passes through to xterm)', () => {
  let writeText: ReturnType<typeof vi.fn>
  const termWithSelection = (selection: string): Terminal =>
    ({ getSelection: () => selection }) as unknown as Terminal

  beforeEach(() => {
    writeText = vi.fn().mockResolvedValue(undefined)
    Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true })
  })

  test('Ctrl+C is passed through (handler returns true, no clipboard write) so xterm raises SIGINT', () => {
    const event = new KeyboardEvent('keydown', { ctrlKey: true, key: 'c' })
    expect(copyOnCmdC(termWithSelection('selected output'), event)).toBe(true)
    expect(writeText).not.toHaveBeenCalled()
  })

  test('Cmd+C with a selection copies it to the clipboard and consumes the event (returns false)', () => {
    const event = new KeyboardEvent('keydown', { metaKey: true, key: 'c' })
    expect(copyOnCmdC(termWithSelection('selected output'), event)).toBe(false)
    expect(writeText).toHaveBeenCalledWith('selected output')
  })

  test('Cmd+C with NO selection is passed through (returns true, no clipboard write)', () => {
    const event = new KeyboardEvent('keydown', { metaKey: true, key: 'c' })
    expect(copyOnCmdC(termWithSelection(''), event)).toBe(true)
    expect(writeText).not.toHaveBeenCalled()
  })
})
