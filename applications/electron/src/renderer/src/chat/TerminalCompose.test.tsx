import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest'
import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { TerminalCompose } from './TerminalCompose'

describe('TerminalCompose — the renderer-owned cooked-mode line', () => {
  beforeEach(() => window.localStorage.clear())
  afterEach(() => {
    cleanup()
    window.localStorage.clear()
  })

  test('Enter submits the line to the caller and clears the compose', async () => {
    const user = userEvent.setup()
    const onSubmit = vi.fn()
    render(<TerminalCompose composeKey="t1" onSubmit={onSubmit} />)
    const input = screen.getByLabelText('terminal input') as HTMLTextAreaElement

    await user.type(input, 'ls -la')
    await user.keyboard('{Enter}')

    expect(onSubmit).toHaveBeenCalledWith('ls -la')
    expect(input.value).toBe('')
  })

  test('the submitted line is VERBATIM — no trim (a shell line owns its own whitespace)', async () => {
    const user = userEvent.setup()
    const onSubmit = vi.fn()
    render(<TerminalCompose composeKey="t2" onSubmit={onSubmit} />)
    const input = screen.getByLabelText('terminal input') as HTMLTextAreaElement

    await user.type(input, '  echo hi  ')
    await user.keyboard('{Enter}')

    expect(onSubmit).toHaveBeenCalledWith('  echo hi  ')
  })

  test('an empty Enter still submits — a bare prompt advances, matching the kernel cooked line', async () => {
    const user = userEvent.setup()
    const onSubmit = vi.fn()
    render(<TerminalCompose composeKey="t3" onSubmit={onSubmit} />)
    const input = screen.getByLabelText('terminal input') as HTMLTextAreaElement

    await user.click(input)
    await user.keyboard('{Enter}')

    expect(onSubmit).toHaveBeenCalledWith('')
  })

  test('reuses the readline editing engine — Ctrl-U kills to line start', async () => {
    const user = userEvent.setup()
    const onSubmit = vi.fn()
    render(<TerminalCompose composeKey="t4" onSubmit={onSubmit} />)
    const input = screen.getByLabelText('terminal input') as HTMLTextAreaElement

    await user.type(input, 'rm -rf important')
    await user.keyboard('{Control>}u{/Control}')

    expect(input.value).toBe('')
  })

  test('↑ recalls the prior submitted line — per-terminal command history', async () => {
    const user = userEvent.setup()
    const onSubmit = vi.fn()
    render(<TerminalCompose composeKey="t5" onSubmit={onSubmit} />)
    const input = screen.getByLabelText('terminal input') as HTMLTextAreaElement

    await user.type(input, 'git status')
    await user.keyboard('{Enter}')
    expect(input.value).toBe('')

    await user.keyboard('{ArrowUp}')

    expect(input.value).toBe('git status')
  })
})
