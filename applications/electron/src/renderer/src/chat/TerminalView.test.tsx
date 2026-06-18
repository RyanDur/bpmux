import { afterEach, beforeEach, describe, expect, test } from 'vitest'
import { cleanup, render, screen } from '@testing-library/react'
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
