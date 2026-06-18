import { describe, expect, test } from 'vitest'
import { renderToStaticMarkup } from 'react-dom/server'
import AppShell from './AppShell'
import type { ConnectionStatus } from './transport/types'

const renderHeader = (status: ConnectionStatus): string => {
  return renderToStaticMarkup(
    <AppShell status={status} messages={[]} onSend={() => {}} />
  )
}

describe('Header title-as-state-canvas (Pictor S3 #1449 rework)', () => {
  test('title carries app name + state text + connected-state-class when status=connected', () => {
    const html = renderHeader('connected')
    expect(html).toContain('data-test="titlebar-app"')
    expect(html).toContain('data-status="connected"')
    expect(html).toMatch(/data-test="titlebar-app-state"[^>]*>online</)
    expect(html).toContain('titlebar-app--connected')
    expect(html).toContain('aria-label="bpmux online"')
  })

  test('title state-text shows "reconnecting" for connecting class (Scenario 2)', () => {
    const html = renderHeader('connecting')
    expect(html).toMatch(/data-test="titlebar-app-state"[^>]*>reconnecting</)
    expect(html).toContain('titlebar-app--connecting')
    expect(html).toContain('aria-label="bpmux reconnecting"')
  })

  test('title state-text shows "offline" for disconnected class', () => {
    const html = renderHeader('disconnected')
    expect(html).toMatch(/data-test="titlebar-app-state"[^>]*>offline</)
    expect(html).toContain('titlebar-app--disconnected')
    expect(html).toContain('aria-label="bpmux offline"')
  })

  test('CI indicator renders without redundant text wrapper (Scenario 4 — CiGlyph self-labels)', () => {
    const html = renderHeader('connected')
    expect(html).toContain('data-test="titlebar-ci"')
    expect(html).not.toMatch(/data-test="titlebar-ci"[^>]*>[\s\S]*?<small[^>]*>CI<\/small>/)
  })

  test('Board indicator NOT in titlebar — relocated to Board column-header per Scenario 5', () => {
    const html = renderHeader('connected')
    expect(html).not.toContain('data-test="titlebar-board"')
  })

  test('right-side cluster does NOT carry density-toggle or lanes-toggle (retired)', () => {
    const html = renderHeader('connected')
    expect(html).not.toContain('data-test="titlebar-density-toggle"')
    expect(html).not.toContain('data-test="titlebar-lanes-toggle"')
  })

  test('standalone "● online" indicator is NOT present — folded into title per Scenario 2', () => {
    const html = renderHeader('connected')
    expect(html).not.toContain('data-test="titlebar-connection"')
    expect(html).not.toContain('data-test="titlebar-connection-label"')
  })

  test('multi-modal signal — title text + class together (WCAG 2.1 SC 1.4.1; Scenario 8)', () => {
    const html = renderHeader('disconnected')
    expect(html).toContain('titlebar-app--disconnected')
    expect(html).toContain('>offline<')
  })
})
