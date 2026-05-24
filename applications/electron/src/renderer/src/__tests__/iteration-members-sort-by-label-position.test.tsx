import { afterEach, describe, expect, inject, it } from 'vitest'
import { within } from '@testing-library/react'

import { renderApp, type RenderAppResult } from '../../../../test/fixtures/renderApp'
import { seedStories } from '../../../../test/fixtures/seedMessages'

describe('#1339 — iteration members sort by iteration-{Name}.{position} label', () => {
  let rendered: RenderAppResult | null = null

  afterEach(() => {
    rendered?.cleanup()
    rendered = null
  })

  it('member stories appear in label-position order inside the iteration container body', async () => {
    const port = inject('bpmuxBackendPort')
    const authToken = inject('bpmuxAuthToken')
    const apiBase = `http://127.0.0.1:${port}`

    await seedStories(apiBase, authToken, [
      {
        number: 7401,
        title: 'Iteration Aquila',
        story_type: 'iteration',
        initial_state: 'icebox'
      },
      {
        number: 7411,
        title: 'story alpha',
        story_type: 'feature',
        initial_state: 'icebox',
        labels: ['parent-iteration-#7401.200']
      },
      {
        number: 7412,
        title: 'story bravo',
        story_type: 'feature',
        initial_state: 'icebox',
        labels: ['parent-iteration-#7401.100']
      },
      {
        number: 7413,
        title: 'story charlie',
        story_type: 'feature',
        initial_state: 'icebox',
        labels: ['parent-iteration-#7401.300']
      }
    ])

    rendered = renderApp()

    const container = await rendered.findByTestId('board-iteration-7401', undefined, {
      timeout: 15_000
    })
    const memberRows = within(container).getAllByTestId(/^board-story-\d+$/)
    const memberNumbers = memberRows.map((el) => Number(el.getAttribute('data-iteration-member')))
    expect(memberNumbers).toEqual([7412, 7411, 7413])
  }, 25_000)
})
