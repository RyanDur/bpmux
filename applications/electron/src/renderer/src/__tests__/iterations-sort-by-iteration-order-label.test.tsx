import { afterEach, describe, expect, inject, it } from 'vitest'

import { renderApp, type RenderAppResult } from '../../../../test/fixtures/renderApp'
import { seedStories } from '../../../../test/fixtures/seedMessages'

describe('#1339 step 2g-a — iteration containers sort by iteration-order.{N} label', () => {
  let rendered: RenderAppResult | null = null

  afterEach(() => {
    rendered?.cleanup()
    rendered = null
  })

  it(
    'iteration with iteration-order.200 renders AFTER iteration with iteration-order.100',
    async () => {
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
        }
      ])

      rendered = renderApp()

      await rendered.findByTestId('board-iteration-8001', undefined, { timeout: 15_000 })
      await rendered.findByTestId('board-iteration-8002', undefined, { timeout: 15_000 })

      const iterationsList = document.querySelector('.board-iterations-list')
      expect(iterationsList).toBeTruthy()
      const renderedIterationIds = Array.from(
        iterationsList!.querySelectorAll('[data-test^="board-iteration-"]')
      )
        .map((el) => el.getAttribute('data-test'))
        .filter(
          (t) => t !== null && /^board-iteration-\d+$/.test(t)
        ) as string[]

      const hydraIdx = renderedIterationIds.indexOf('board-iteration-8002')
      const lynxIdx = renderedIterationIds.indexOf('board-iteration-8001')
      expect(hydraIdx).toBeGreaterThanOrEqual(0)
      expect(lynxIdx).toBeGreaterThanOrEqual(0)
      expect(hydraIdx).toBeLessThan(lynxIdx)
    },
    25_000
  )
})
