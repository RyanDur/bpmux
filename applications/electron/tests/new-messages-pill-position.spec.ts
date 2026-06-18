import { test, expect, type Page } from '@playwright/test'
import { launchWithMockGh, waitForMessagesLoaded } from './helpers/launch'
import { ChatPage } from './pages/chatPage'

const SEED_COUNT = 60
const CENTER_TOLERANCE_PX = 8
const SETTLE_MS = 400

const seedMessages = (count: number): Array<Record<string, unknown>> =>
  Array.from({ length: count }, (_, i) => {
    const minute = String(i % 60).padStart(2, '0')
    const hour = String(10 + Math.floor(i / 60)).padStart(2, '0')
    return {
      from: 'dev:claude-code',
      text: `seed message ${i}`,
      ts: `2026-05-03T${hour}:${minute}:00Z`,
      message_type: 'chat'
    }
  })

const scrollToVisualTopAwayFromBottom = async (win: Page): Promise<void> => {
  const ul = new ChatPage(win).messageScroller()
  await expect(ul).toBeVisible()
  await ul.evaluate((el) => {
    el.scrollTop = -(el.scrollHeight - el.clientHeight)
    el.dispatchEvent(new Event('scroll'))
  })
}

const broadcastLivePeerMessage = async (backendPort: number, text: string): Promise<void> => {
  const res = await fetch(`http://127.0.0.1:${backendPort}/api/messages`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ from: 'pm:claude-code', text, message_type: 'chat' })
  })
  expect(res.ok, `live peer message POST should succeed (got ${res.status})`).toBeTruthy()
}

interface PillGeometry {
  centeredWithinWrap: boolean
  inBottomHalfOfWrap: boolean
  overlapsFilterPills: boolean
}

const measurePillGeometry = async (win: Page): Promise<PillGeometry> => {
  const chat = new ChatPage(win)
  const pill = chat.newMessagesPill()
  await expect(pill).toBeVisible()
  const pillBox = await pill.boundingBox()
  const wrapBox = await win.locator('.message-list-wrap').boundingBox()
  const filterPillsBox = await chat.filterRegion().boundingBox()
  if (pillBox === null || wrapBox === null) {
    throw new Error('pill or message-list-wrap has no bounding box')
  }
  const pillCenterX = pillBox.x + pillBox.width / 2
  const wrapCenterX = wrapBox.x + wrapBox.width / 2
  const gapBelowPill = wrapBox.y + wrapBox.height - (pillBox.y + pillBox.height)
  const overlapsFilterPills =
    filterPillsBox !== null &&
    Math.min(pillBox.x + pillBox.width, filterPillsBox.x + filterPillsBox.width) -
      Math.max(pillBox.x, filterPillsBox.x) >
      0 &&
    Math.min(pillBox.y + pillBox.height, filterPillsBox.y + filterPillsBox.height) -
      Math.max(pillBox.y, filterPillsBox.y) >
      0
  return {
    centeredWithinWrap: Math.abs(pillCenterX - wrapCenterX) <= CENTER_TOLERANCE_PX,
    inBottomHalfOfWrap: gapBelowPill >= 0 && gapBelowPill <= wrapBox.height / 2,
    overlapsFilterPills
  }
}

const driveScrolledUpWithNewMessageBelow = async (
  win: Page,
  backendPort: number,
  marker: string
): Promise<void> => {
  await waitForMessagesLoaded(win, 'seed message 59')
  await expect(new ChatPage(win).filterRegion()).toBeVisible()
  await scrollToVisualTopAwayFromBottom(win)
  await broadcastLivePeerMessage(backendPort, marker)
  await expect(new ChatPage(win).newMessagesPill()).toBeVisible({ timeout: 5000 })
}

test.describe('#1512 new-messages pill — bottom-center, never top-left/overlapping filter-pills', () => {
  test('default width: pill is bottom-center and does not overlap the filter-pills row, from mount through settle', async () => {
    const { app, win, mockGh, backendPort } = await launchWithMockGh({
      seedMessages: seedMessages(SEED_COUNT)
    })
    try {
      await driveScrolledUpWithNewMessageBelow(win, backendPort, 'live pm message default')

      const atMount = await measurePillGeometry(win)
      expect(atMount.overlapsFilterPills, 'pill must not overlap filter-pills at mount').toBe(false)
      expect(atMount.inBottomHalfOfWrap, 'pill must sit in the bottom half at mount').toBe(true)
      expect(atMount.centeredWithinWrap, 'pill must be horizontally centered at mount').toBe(true)

      await win.waitForTimeout(SETTLE_MS)

      const afterSettle = await measurePillGeometry(win)
      expect(
        afterSettle.overlapsFilterPills,
        'pill must not overlap filter-pills after settle'
      ).toBe(false)
      expect(afterSettle.inBottomHalfOfWrap, 'pill must sit in the bottom half after settle').toBe(
        true
      )
      expect(
        afterSettle.centeredWithinWrap,
        'pill must be horizontally centered after settle'
      ).toBe(true)
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('narrow width (480): pill is bottom-center and does not overlap the filter-pills row, from mount through settle', async () => {
    const { app, win, mockGh, backendPort } = await launchWithMockGh({
      seedMessages: seedMessages(SEED_COUNT)
    })
    try {
      await win.setViewportSize({ width: 480, height: 600 })
      await driveScrolledUpWithNewMessageBelow(win, backendPort, 'live pm message narrow')

      const atMount = await measurePillGeometry(win)
      expect(
        atMount.overlapsFilterPills,
        'narrow: pill must not overlap filter-pills at mount'
      ).toBe(false)
      expect(atMount.inBottomHalfOfWrap, 'narrow: pill must sit in the bottom half at mount').toBe(
        true
      )
      expect(
        atMount.centeredWithinWrap,
        'narrow: pill must be horizontally centered at mount'
      ).toBe(true)

      await win.waitForTimeout(SETTLE_MS)

      const afterSettle = await measurePillGeometry(win)
      expect(
        afterSettle.overlapsFilterPills,
        'narrow: pill must not overlap filter-pills after settle'
      ).toBe(false)
      expect(
        afterSettle.inBottomHalfOfWrap,
        'narrow: pill must sit in the bottom half after settle'
      ).toBe(true)
      expect(
        afterSettle.centeredWithinWrap,
        'narrow: pill must be horizontally centered after settle'
      ).toBe(true)
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
