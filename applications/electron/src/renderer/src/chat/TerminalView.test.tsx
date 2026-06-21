import { afterEach, beforeEach, describe, expect, test } from 'vitest'
import { cleanup, render, screen, waitFor } from '@testing-library/react'
import { TerminalView } from './TerminalView'
import {
  deliverServerFrameForTest,
  resetSessionForTest,
  encodeKeystroke
} from '../transport/session'

const osc7Sequence = (path: string): string =>
  `${String.fromCharCode(27)}]7;file://${path}${String.fromCharCode(7)}`

const ptyOutputFrame = (terminalId: string, raw: string): string =>
  JSON.stringify({ type: 'pty_output', terminal_id: terminalId, data: encodeKeystroke(raw) })

const ptyModeFrame = (terminalId: string, bracketedPaste: boolean): string =>
  JSON.stringify({ type: 'pty_mode', terminal_id: terminalId, bracketed_paste: bracketedPaste })

describe('#1597 TerminalView — the header follows an OSC-7 cwd update from the PTY stream', () => {
  beforeEach(() => resetSessionForTest())
  afterEach(() => {
    cleanup()
    resetSessionForTest()
  })

  test('an OSC-7 file:// sequence arriving as pty_output updates the header to the new dir', async () => {
    render(<TerminalView terminalId="osc7-unit" cwd="/home/start" isActive={true} />)
    expect(screen.getByText(/\/home\/start/)).toBeTruthy()

    deliverServerFrameForTest(ptyOutputFrame('osc7-unit', osc7Sequence('/new/live/dir')))

    expect(await screen.findByText(/\/new\/live\/dir/)).toBeTruthy()
  })
})

describe('#1656 TerminalView — the input-mode mirrors the live bracketed-paste state from pty_mode', () => {
  beforeEach(() => resetSessionForTest())
  afterEach(() => {
    cleanup()
    resetSessionForTest()
  })

  test('defaults to cooked, flips to raw on a bracketed_paste frame, and back to cooked', async () => {
    render(<TerminalView terminalId="mode-unit" cwd="/home/start" isActive={true} />)
    const screenRegion = screen.getByLabelText('terminal output')
    expect(screenRegion.getAttribute('data-input-mode')).toBe('cooked')

    deliverServerFrameForTest(ptyModeFrame('mode-unit', true))
    await waitFor(() => expect(screenRegion.getAttribute('data-input-mode')).toBe('raw'))

    deliverServerFrameForTest(ptyModeFrame('mode-unit', false))
    await waitFor(() => expect(screenRegion.getAttribute('data-input-mode')).toBe('cooked'))
  })

  test('the renderer-owned compose-line shows in cooked mode and hides when the agent goes raw', async () => {
    render(<TerminalView terminalId="compose-unit" cwd="/home/start" isActive={true} />)
    expect(screen.getByLabelText('terminal input')).toBeTruthy()

    deliverServerFrameForTest(ptyModeFrame('compose-unit', true))
    await waitFor(() => expect(screen.queryByLabelText('terminal input')).toBeNull())

    deliverServerFrameForTest(ptyModeFrame('compose-unit', false))
    expect(await screen.findByLabelText('terminal input')).toBeTruthy()
  })
})
