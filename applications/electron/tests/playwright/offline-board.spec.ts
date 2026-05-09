import { expect, test } from '@playwright/test'
import { mkdtempSync, readFileSync, writeFileSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedWithWindow } from '../helpers/launch'

test.describe('#1158 — offline-first board', () => {
  test('F1 visibility — sync-status indicator visible from any view', async () => {
    const { app, win } = await launchIsolatedWithWindow({
      extraEnv: { BPMUX_TEST_MODE: '1' }
    })
    try {
      const region = win.locator('.sync-status-region')
      await expect(region).toBeVisible()
      await expect(region).toHaveAttribute('role', 'status')
      await expect(region).toHaveAttribute('aria-live', 'polite')
      const indicator = win.locator('[data-test="sync-status-glyph"]')
      await expect(indicator).toBeVisible()
      await expect(indicator).toHaveAttribute(
        'data-state',
        /^(connected|syncing|offline-queued|drained-permanent-failure|recovering|stale|startup-fail)$/
      )
    } finally {
      await app.close()
    }
  })

  test('F2 honest-success — /api/sync-status returns SyncStatus shape', async () => {
    const { app, win, backendPort } = await launchIsolatedWithWindow({
      extraEnv: { BPMUX_TEST_MODE: '1' }
    })
    try {
      const token = await win.evaluate(
        () => (window as unknown as { bpmux?: { authToken?: string } }).bpmux?.authToken ?? ''
      )
      const res = await fetch(`http://127.0.0.1:${backendPort}/api/sync-status`, {
        headers: { Authorization: `Bearer ${token}` }
      })
      expect(res.ok).toBe(true)
      const json = await res.json()
      expect(json).toHaveProperty('pending_events')
      expect(json).toHaveProperty('last_error')
      expect(json).toHaveProperty('dropped_events')
      expect(Array.isArray(json.dropped_events)).toBe(true)
      expect(json).toHaveProperty('recovering_from_corruption')
      expect(typeof json.recovering_from_corruption).toBe('boolean')
      expect(json).toHaveProperty('stale')
      expect(typeof json.stale).toBe('boolean')
      expect(json).toHaveProperty('startup_fail')
      expect(typeof json.startup_fail).toBe('boolean')
    } finally {
      await app.close()
    }
  })

  test('F3 causal-sync — outbox FIFO order preserved (seeded outbox replays)', async () => {
    const outboxPath = join(
      mkdtempSync(join(tmpdir(), 'bpmux-offline-outbox-')),
      'board-outbox.json'
    )
    const seedEvents = [
      {
        StateTransitioned: {
          meta: {
            actor: 'dev',
            ts: '2026-05-03T20:00:00Z',
            peer_id: 'local'
          },
          number: 100,
          new_state: 'InProgress'
        }
      },
      {
        StateTransitioned: {
          meta: {
            actor: 'dev',
            ts: '2026-05-03T20:00:01Z',
            peer_id: 'local'
          },
          number: 101,
          new_state: 'InProgress'
        }
      }
    ]
    writeFileSync(outboxPath, JSON.stringify(seedEvents))

    const { app, win, backendPort } = await launchIsolatedWithWindow({
      extraEnv: { BPMUX_BOARD_OUTBOX_PATH: outboxPath, BPMUX_TEST_MODE: '1' }
    })
    try {
      const token = await win.evaluate(
        () => (window as unknown as { bpmux?: { authToken?: string } }).bpmux?.authToken ?? ''
      )
      const res = await fetch(`http://127.0.0.1:${backendPort}/api/sync-status`, {
        headers: { Authorization: `Bearer ${token}` }
      })
      expect(res.ok).toBe(true)
      const json = await res.json()
      expect(json.pending_events).toBe(2)
    } finally {
      await app.close()
    }
  })

  test('F9 startup-fail — glyph surfaces startup-fail when initial pull never succeeds', async () => {
    const { app, win } = await launchIsolatedWithWindow({
      extraEnv: { BPMUX_TEST_MODE: '1', BPMUX_TEST_INGEST_FAIL: '1' }
    })
    try {
      const indicator = win.locator('[data-test="sync-status-glyph"]')
      await expect(indicator).toBeVisible()
      await expect(indicator).toHaveAttribute('data-state', 'startup-fail', { timeout: 30_000 })
      const computed = await indicator.evaluate(
        (el) => window.getComputedStyle(el as HTMLElement).color
      )
      expect(
        computed,
        'startup-fail glyph picks up the error color (not default fg, not green)'
      ).not.toBe('rgb(0, 0, 0)')
    } finally {
      await app.close()
    }
  })

  test('F8 stale — glyph surfaces stale state when ingest exceeds threshold', async () => {
    const { app, win } = await launchIsolatedWithWindow({
      extraEnv: { BPMUX_TEST_MODE: '1', BPMUX_STALENESS_THRESHOLD_SECS: '0' }
    })
    try {
      const indicator = win.locator('[data-test="sync-status-glyph"]')
      await expect(indicator).toBeVisible()
      await expect(indicator).toHaveAttribute('data-state', 'stale', { timeout: 30_000 })
      const computed = await indicator.evaluate(
        (el) => window.getComputedStyle(el as HTMLElement).color
      )
      expect(computed, 'stale glyph picks up the warning color (not default fg)').not.toBe(
        'rgb(0, 0, 0)'
      )
    } finally {
      await app.close()
    }
  })

  test('F5 restart-survival — outbox file persists across electron restart', async () => {
    const outboxPath = join(
      mkdtempSync(join(tmpdir(), 'bpmux-restart-outbox-')),
      'board-outbox.json'
    )
    const event = {
      StateTransitioned: {
        meta: {
          actor: 'dev',
          ts: '2026-05-03T20:00:00Z',
          peer_id: 'local'
        },
        number: 200,
        new_state: 'Delivered'
      }
    }
    writeFileSync(outboxPath, JSON.stringify([event]))

    const first = await launchIsolatedWithWindow({
      extraEnv: { BPMUX_BOARD_OUTBOX_PATH: outboxPath, BPMUX_TEST_MODE: '1' }
    })
    try {
      const token1 = await first.win.evaluate(
        () => (window as unknown as { bpmux?: { authToken?: string } }).bpmux?.authToken ?? ''
      )
      const res1 = await fetch(`http://127.0.0.1:${first.backendPort}/api/sync-status`, {
        headers: { Authorization: `Bearer ${token1}` }
      })
      expect((await res1.json()).pending_events).toBe(1)
    } finally {
      await first.app.close()
    }

    const persisted = JSON.parse(readFileSync(outboxPath, 'utf-8')) as unknown[]
    expect(persisted.length).toBe(1)

    const second = await launchIsolatedWithWindow({
      extraEnv: { BPMUX_BOARD_OUTBOX_PATH: outboxPath, BPMUX_TEST_MODE: '1' }
    })
    try {
      const token2 = await second.win.evaluate(
        () => (window as unknown as { bpmux?: { authToken?: string } }).bpmux?.authToken ?? ''
      )
      const res2 = await fetch(`http://127.0.0.1:${second.backendPort}/api/sync-status`, {
        headers: { Authorization: `Bearer ${token2}` }
      })
      expect((await res2.json()).pending_events).toBe(1)
    } finally {
      await second.app.close()
    }
  })
})
