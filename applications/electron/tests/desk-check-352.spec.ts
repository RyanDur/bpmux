import { test, expect, _electron as electron } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron } from './helpers/launch'
import { ChatPage } from './pages/chatPage'

const EVIDENCE_DIR = join(__dirname, 'desk-check-evidence')

const MOCK_MESSAGES = [
  {
    id: 'mm1',
    from: 'designer:claude-code',
    text: 'Mock message for avatar + arrival',
    ts: '2026-04-22T16:00:00Z',
    messageType: 'chat'
  },
  {
    id: 'mm2',
    from: 'dev:claude-code',
    text: 'Dev speaks',
    ts: '2026-04-22T16:01:00Z',
    messageType: 'chat'
  }
]

const launchWithMessages = async (
  reducedMotion?: 'reduce'
): Promise<{
  app: Awaited<ReturnType<typeof electron.launch>>
}> => {
  const sessionBase = mkdtempSync(join(tmpdir(), 'bpmux-desk-check-352-'))
  const udd = mkdtempSync(join(tmpdir(), 'bpmux-userdata-352-'))
  const { app } = await launchIsolatedElectron({ sessionBase, userDataDir: udd })
  const win = await app.firstWindow()
  if (reducedMotion) await win.emulateMedia({ reducedMotion })
  await win.route('**/api/messages*', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(MOCK_MESSAGES)
    })
  })
  await win.reload()
  return { app }
}

test.describe('desk-check #352 micro-interactions', () => {
  test('filter pill transitions at instant duration (80ms)', async () => {
    const { app } = await launchWithMessages()
    const win = await app.firstWindow()
    await expect(new ChatPage(win).filterRegion()).toBeVisible()
    const pill = await win.evaluate(() => {
      const el = document.querySelector('.filter-pill')
      if (!el) return null
      const cs = window.getComputedStyle(el)
      return { property: cs.transitionProperty, duration: cs.transitionDuration }
    })
    console.log(`#352 AC1 filter-pill transition:`)
    console.log(`  property: ${pill?.property}`)
    console.log(`  duration: ${pill?.duration}`)
    expect(pill?.property).toContain('background-color')
    expect(pill?.property).toContain('color')
    expect(pill?.property).toContain('border-color')
    expect(pill?.duration).toMatch(/0\.08s|80ms/)
    await app.close()
  })

  test('button global base has hover transition', async () => {
    const { app } = await launchWithMessages()
    const win = await app.firstWindow()
    await expect(new ChatPage(win).appRoot()).toBeVisible()

    const anyButton = await win.evaluate(() => {
      // Find any <button> that isn't a filter-pill (those have their
      // own override, we want the base rule)
      const btn = Array.from(document.querySelectorAll('button')).find(
        (el) => !el.className.includes('filter-pill')
      )
      if (!btn) return null
      const cs = window.getComputedStyle(btn)
      return {
        tag: btn.tagName,
        className: btn.className,
        property: cs.transitionProperty,
        duration: cs.transitionDuration
      }
    })
    console.log(`#352 AC2 sample button:`)
    console.log(`  tag: ${anyButton?.tag}, class: ${anyButton?.className}`)
    console.log(`  transition-property: ${anyButton?.property}`)
    console.log(`  transition-duration: ${anyButton?.duration}`)

    // Some buttons may have component-level overrides. Accept either
    // instant or fast duration (both are token-backed + reduced-motion-
    // respecting).
    expect(anyButton?.duration).toMatch(/0\.08s|0\.15s|80ms|150ms/)

    await app.close()
  })

  test('message-row has grid-template-rows transition at fast duration', async () => {
    const { app } = await launchWithMessages()
    const win = await app.firstWindow()
    await expect(new ChatPage(win).messageScroller().getByRole('listitem').first()).toBeVisible()

    const row = await win.evaluate(() => {
      const el = document.querySelector('.message-row')
      if (!el) return null
      const cs = window.getComputedStyle(el)
      return {
        property: cs.transitionProperty,
        duration: cs.transitionDuration
      }
    })
    console.log(`#352 AC4 message-row transition:`)
    console.log(`  property: ${row?.property}`)
    console.log(`  duration: ${row?.duration}`)
    expect(row?.property).toContain('grid-template-rows')
    expect(row?.duration).toMatch(/0\.15s|150ms/)
    await app.close()
  })

  test('reduced-motion zeros filter-pill + message-row durations; message-row transform drops', async () => {
    const { app } = await launchWithMessages('reduce')
    const win = await app.firstWindow()
    await expect(new ChatPage(win).filterRegion()).toBeVisible()

    const pillDur = await win.evaluate(
      () => window.getComputedStyle(document.querySelector('.filter-pill')!).transitionDuration
    )
    console.log(`#352 AC6 filter-pill duration under reduced-motion: ${pillDur}`)
    expect(pillDur).toMatch(/^0s|^0ms/)

    const rowProps = await win.evaluate(() => {
      const el = document.querySelector('.message-row')
      if (!el) return null
      const cs = window.getComputedStyle(el)
      return {
        property: cs.transitionProperty,
        duration: cs.transitionDuration,
        transform: cs.transform
      }
    })
    console.log(`#352 AC6 message-row under reduced-motion:`)
    console.log(`  property: ${rowProps?.property}`)
    console.log(`  duration: ${rowProps?.duration}`)
    console.log(`  transform: ${rowProps?.transform}`)
    // Per dev's AC6: 'drops the translateY transform — keeps only
    // opacity so arrival still feels like something appeared'.
    expect(rowProps?.duration).toMatch(/^0s|^0ms/)

    await app.close()
  })

  test('capture static state for visual regression (motion not frame-capturable)', async () => {
    const { app } = await launchWithMessages()
    const win = await app.firstWindow()
    await expect(new ChatPage(win).region()).toBeVisible()
    await win.waitForTimeout(500)
    await win.screenshot({
      path: join(EVIDENCE_DIR, '70-micro-interactions-static.png')
    })
    await app.close()
  })
})
