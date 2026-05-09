import { test, expect } from '@playwright/test'
import { mkdtempSync, writeFileSync, existsSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron } from './helpers/launch'

interface BoardResponse {
  stories: Array<{ number: number; title: string }>
}

const uniqueMarker = (axis: string): string =>
  `LEAK_CANARY_${axis}_${Date.now()}_${Math.random().toString(36).slice(2)}`

test.describe('#1216 env-leak canary — launchIsolatedElectron isolation', () => {
  test('BPMUX_BOARD_STORE_PATH leak: process.env value seeded with marker must not surface in spawned backend', async () => {
    const leakyDir = mkdtempSync(join(tmpdir(), 'canary-leak-store-'))
    const leakyPath = join(leakyDir, 'board-store.json')
    const marker = uniqueMarker('STORE')
    writeFileSync(
      leakyPath,
      JSON.stringify({
        stories: [
          {
            number: 99999,
            title: marker,
            state: 'icebox',
            story_type: 'feature',
            tags: [],
            filed_at: '2026-01-01T00:00:00Z',
            filed_by: 'leak-canary',
            raw_labels: [],
            is_closed: false,
            assignees: [],
            body: '',
            created_at: '2026-01-01T00:00:00Z',
            parent_epic: null,
            rework_comment: null
          }
        ]
      })
    )

    const priorEnvValue = process.env.BPMUX_BOARD_STORE_PATH
    process.env.BPMUX_BOARD_STORE_PATH = leakyPath
    try {
      const { app, backendPort } = await launchIsolatedElectron({})
      const win = await app.firstWindow()
      const authToken = await win.evaluate(
        () =>
          (window as unknown as { bpmux?: { authToken?: string } }).bpmux
            ?.authToken ?? ''
      )
      const board = (await win.evaluate(
        async ({ apiBase, token }) => {
          const res = await fetch(`${apiBase}/api/board`, {
            headers: { Authorization: `Bearer ${token}` }
          })
          return res.json()
        },
        { apiBase: `http://127.0.0.1:${backendPort}`, token: authToken }
      )) as BoardResponse

      const found = board.stories.find((s) => s.title.includes(marker))
      expect(
        found,
        `spawned bpmux must NOT inherit process.env.BPMUX_BOARD_STORE_PATH; ` +
          `marker '${marker}' from leaky-path '${leakyPath}' surfaced in ` +
          `spawned backend's board API`
      ).toBeUndefined()

      await app.close()
    } finally {
      if (priorEnvValue === undefined) delete process.env.BPMUX_BOARD_STORE_PATH
      else process.env.BPMUX_BOARD_STORE_PATH = priorEnvValue
    }
  })

  test('BPMUX_LOG leak: spawned backend diag log path must be launch tempdir, not process.env value nor ~/.bpmux/', async () => {
    const leakyDir = mkdtempSync(join(tmpdir(), 'canary-leak-log-'))
    const leakyLogPath = join(leakyDir, 'leaky-bpmux-test-diag.log')

    const priorEnvValue = process.env.BPMUX_LOG
    process.env.BPMUX_LOG = leakyLogPath
    try {
      const { app } = await launchIsolatedElectron({})
      await app.firstWindow()

      expect(
        existsSync(leakyLogPath),
        `spawned bpmux must NOT write to process.env.BPMUX_LOG ` +
          `(${leakyLogPath}); launchIsolatedElectron's per-test ` +
          `BPMUX_LOG override must win`
      ).toBe(false)

      await app.close()
    } finally {
      if (priorEnvValue === undefined) delete process.env.BPMUX_LOG
      else process.env.BPMUX_LOG = priorEnvValue
    }
  })

  test('BPMUX_BOARD_OUTBOX_PATH leak: pre-seeded outbox must not surface as pending events in spawned backend', async () => {
    const leakyDir = mkdtempSync(join(tmpdir(), 'canary-leak-outbox-'))
    const leakyOutboxPath = join(leakyDir, 'board-outbox.json')
    const marker = uniqueMarker('OUTBOX')
    writeFileSync(
      leakyOutboxPath,
      JSON.stringify([
        {
          NewShape: {
            id: 1,
            event: {
              FreeCommentAppended: {
                meta: {
                  actor: marker,
                  ts: '2026-01-01T00:00:00Z',
                  peer_id: 'leak-canary'
                },
                number: 99999,
                body: 'leak-outbox-canary'
              }
            },
            pre_state: null,
            last_classification: null
          }
        }
      ])
    )

    const priorEnvValue = process.env.BPMUX_BOARD_OUTBOX_PATH
    process.env.BPMUX_BOARD_OUTBOX_PATH = leakyOutboxPath
    try {
      const { app, backendPort } = await launchIsolatedElectron({})
      const win = await app.firstWindow()
      const authToken = await win.evaluate(
        () =>
          (window as unknown as { bpmux?: { authToken?: string } }).bpmux
            ?.authToken ?? ''
      )
      const status = (await win.evaluate(
        async ({ apiBase, token }) => {
          const res = await fetch(`${apiBase}/api/sync-status`, {
            headers: { Authorization: `Bearer ${token}` }
          })
          return res.json()
        },
        { apiBase: `http://127.0.0.1:${backendPort}`, token: authToken }
      )) as { pending_events: number }

      expect(
        status.pending_events,
        `spawned bpmux must NOT inherit process.env.BPMUX_BOARD_OUTBOX_PATH; ` +
          `pre-seeded outbox at '${leakyOutboxPath}' (1 pending event) ` +
          `propagated into spawned backend's pending count (${status.pending_events})`
      ).toBe(0)

      await app.close()
    } finally {
      if (priorEnvValue === undefined)
        delete process.env.BPMUX_BOARD_OUTBOX_PATH
      else process.env.BPMUX_BOARD_OUTBOX_PATH = priorEnvValue
    }
  })

})
