import { afterEach, describe, expect, inject, it } from 'vitest'
import { fireEvent, within } from '@testing-library/react'

import { renderApp, type RenderAppResult } from '../../../../test/fixtures/renderApp'
import { seedStories } from '../../../../test/fixtures/seedMessages'

describe('#1339 step 2d — dragged member carries data-dragging during drag gesture', () => {
  let rendered: RenderAppResult | null = null

  afterEach(() => {
    rendered?.cleanup()
    rendered = null
  })

  it('onDragStart sets data-dragging="true" on the source row; onDragEnd clears it', async () => {
    const port = inject('bpmuxBackendPort')
    const authToken = inject('bpmuxAuthToken')
    const apiBase = `http://127.0.0.1:${port}`

    await seedStories(apiBase, authToken, [
      {
        number: 7801,
        title: 'Iteration Lyra',
        story_type: 'iteration',
        initial_state: 'icebox'
      },
      {
        number: 7811,
        title: 'lyra-member',
        story_type: 'feature',
        initial_state: 'icebox',
        labels: ['parent-iteration-#7801.100']
      }
    ])

    rendered = renderApp()

    const container = await rendered.findByTestId('board-iteration-7801', undefined, {
      timeout: 15_000
    })
    const member = within(container).getByTestId('board-story-7811') as HTMLLIElement
    expect(member).toBeTruthy()
    const memberGrip = member.querySelector('[data-test="story-grip"]') as HTMLElement
    expect(memberGrip).toBeTruthy()
    expect(member.getAttribute('data-dragging')).toBeNull()

    const dataStore: Record<string, string> = {}
    const dt: Pick<DataTransfer, 'setData' | 'getData' | 'types' | 'effectAllowed' | 'dropEffect'> =
      {
        setData: (type: string, value: string) => {
          dataStore[type] = value
        },
        getData: (type: string) => dataStore[type] ?? '',
        types: [] as unknown as readonly string[],
        effectAllowed: 'move',
        dropEffect: 'move'
      }
    fireEvent.dragStart(memberGrip, { dataTransfer: dt })
    expect(member.getAttribute('data-dragging')).toBe('true')

    fireEvent.dragEnd(memberGrip, { dataTransfer: dt })
    expect(member.getAttribute('data-dragging')).toBeNull()
  }, 20_000)
})
