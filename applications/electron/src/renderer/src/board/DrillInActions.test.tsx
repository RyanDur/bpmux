import { describe, expect, test } from 'vitest'
import { renderToStaticMarkup } from 'react-dom/server'
import DrillInActions, { type ActionMode } from './DrillInActions'
import type { Story } from './types'
const story = (overrides: Partial<Story>): Story => ({
  id: 800,
  title: 'stub story',
  state: 'acceptance',
  labels: ['Feature'],
  assignees: [],
  created_at: '2026-05-09T00:00:00Z',
  body: '',
  ...overrides
})

const baseProps = {
  apiBase: undefined,
  authToken: undefined,
  mode: 'idle' as ActionMode,
  setMode: () => {},
  onStoryMutated: () => {},
  onBack: () => {},
  onError: () => {}
}

describe('#1355 DrillInActions independence', () => {
  test('renders standalone (substrate-axis) when canAct=true', () => {
    const html = renderToStaticMarkup(
      <DrillInActions {...baseProps} story={story({})} canAct={true} />
    )
    expect(html).toContain('drill-in-actions')
    expect(html).toContain('data-test="drill-in-accept"')
    expect(html).toContain('data-test="drill-in-reject"')
  })

  test('renders nothing when canAct=false', () => {
    const html = renderToStaticMarkup(
      <DrillInActions {...baseProps} story={story({})} canAct={false} />
    )
    expect(html).toBe('')
  })

  test('hides accept/reject menu when mode is not idle', () => {
    const html = renderToStaticMarkup(
      <DrillInActions {...baseProps} story={story({})} canAct={true} mode="rejecting" />
    )
    expect(html).not.toContain('data-test="drill-in-actions"')
  })

  test('renders the bug-question dialog when mode=bug-question', () => {
    const html = renderToStaticMarkup(
      <DrillInActions {...baseProps} story={story({})} canAct={true} mode="bug-question" />
    )
    expect(html).toContain('data-test="drill-in-bug-question"')
    expect(html).toContain('Would you file a bug')
    expect(html).toContain('data-test="drill-in-bug-question-no"')
    expect(html).toContain('data-test="drill-in-bug-question-yes"')
  })

  test('reject dialog is always rendered (showModal toggled via effect)', () => {
    const html = renderToStaticMarkup(
      <DrillInActions {...baseProps} story={story({})} canAct={true} />
    )
    expect(html).toContain('drill-in-reject-form')
    expect(html).toContain('data-test="drill-in-reject-note"')
    expect(html).toContain('data-test="drill-in-reject-send"')
  })

  test('bypass details element is present inside bug-question dialog', () => {
    const html = renderToStaticMarkup(
      <DrillInActions {...baseProps} story={story({})} canAct={true} mode="bug-question" />
    )
    expect(html).toContain('drill-in-bypass')
    expect(html).toContain('data-test="drill-in-bypass-reason"')
    expect(html).toContain('data-test="drill-in-bypass-submit"')
  })
})
