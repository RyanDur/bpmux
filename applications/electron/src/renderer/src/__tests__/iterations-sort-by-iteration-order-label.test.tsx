import { afterEach, describe, expect, inject, it } from 'vitest'

import { renderApp, type RenderAppResult } from '../../../../test/fixtures/renderApp'
import { seedStories } from '../../../../test/fixtures/seedMessages'
import { iterationBoard } from '../board/iterationBoard.page'

describe('#1339 step 2g-a — iteration containers sort by iteration-order.{N} label', () => {
  let rendered: RenderAppResult | null = null

  afterEach(() => {
    rendered?.cleanup()
    rendered = null
  })

  it('iteration with iteration-order.200 renders AFTER iteration with iteration-order.100', async () => {
    const port = inject('bpmuxBackendPort')
    const authToken = inject('bpmuxAuthToken')
    const apiBase = `http://127.0.0.1:${port}`

    await seedStories(apiBase, authToken, [
      {
        number: 8001,
        title: 'Iteration Lynx',
        story_type: 'iteration',
        initial_state: 'icebox',
        labels: ['iteration-order.200']
      },
      {
        number: 8002,
        title: 'Iteration Hydra',
        story_type: 'iteration',
        initial_state: 'icebox',
        labels: ['iteration-order.100']
      },
      {
        number: 8010,
        title: 'lynx-member',
        story_type: 'feature',
        initial_state: 'icebox',
        labels: ['iteration-Lynx.100']
      },
      {
        number: 8011,
        title: 'hydra-member',
        story_type: 'feature',
        initial_state: 'icebox',
        labels: ['iteration-Hydra.100']
      }
    ])

    rendered = renderApp()

    await iterationBoard.container(8001, undefined, { timeout: 15_000 })
    await iterationBoard.container(8002, undefined, { timeout: 15_000 })

    const order = iterationBoard.iterationOrder()
    const hydraIdx = order.indexOf(8002)
    const lynxIdx = order.indexOf(8001)
    expect(hydraIdx).toBeGreaterThanOrEqual(0)
    expect(lynxIdx).toBeGreaterThanOrEqual(0)
    expect(hydraIdx).toBeLessThan(lynxIdx)
  }, 25_000)
})
