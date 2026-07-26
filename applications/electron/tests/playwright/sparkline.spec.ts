import { test, expect } from '@playwright/test'
import { launchIsolatedElectron } from '../helpers/launch'
import { MetricsPage } from '../pages/metricsPage'
import { SparklinePage } from '../pages/sparklinePage'

const daysAgo = (n: number): string => new Date(Date.now() - n * 86_400_000).toISOString()

const fourActivityDaysClimbingThenEasing = [
  { story_id: 1, new_state: 'in-progress', observed_at: daysAgo(4) },
  { story_id: 2, new_state: 'in-progress', observed_at: daysAgo(3) },
  { story_id: 3, new_state: 'in-progress', observed_at: daysAgo(2) },
  { story_id: 1, new_state: 'acceptance', observed_at: daysAgo(1) }
]

test.describe('#1237 WIP-count sparkline reflects real lifecycle history', () => {
  test('renders a multi-point trend derived from the story-transition ledger, with no console errors', async () => {
    const { app, backendPort } = await launchIsolatedElectron({})
    const win = await app.firstWindow()

    const token = await win.evaluate(
      () => (window as unknown as { bpmux?: { authToken?: string } }).bpmux?.authToken ?? ''
    )
    const seeded = await fetch(`http://127.0.0.1:${backendPort}/api/test/seed_transitions`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${token}` },
      body: JSON.stringify(fourActivityDaysClimbingThenEasing)
    })
    expect(seeded.ok).toBe(true)

    const consoleErrors: string[] = []
    const pageErrors: Error[] = []
    win.on('console', (m) => {
      if (m.type() === 'error') consoleErrors.push(m.text())
    })
    win.on('pageerror', (e) => pageErrors.push(e))

    await expect(win.getByRole('main')).toBeVisible()

    const metrics = new MetricsPage(win)
    const sparkline = new SparklinePage(win)
    await win.keyboard.press('Control+Shift+M')
    await expect(metrics.overlay()).toBeVisible()

    await expect(sparkline.svg()).toBeVisible()
    expect(await sparkline.animationName()).toBe('sparkline-draw-in')
    const drawIn = await sparkline.drawInKeyframes()
    expect(drawIn.exists).toBe(true)
    expect(drawIn.from).toBeGreaterThan(0)
    expect(drawIn.to).toBe(0)
    await expect(sparkline.svg()).toHaveAttribute('aria-label', /WIP trend/i)
    await expect(sparkline.polyline()).toBeAttached()
    await expect(sparkline.endpointDot()).toBeAttached()

    expect(await sparkline.polylinePointCount()).toBe(4)

    await expect(sparkline.dataTable()).toBeAttached()

    expect(consoleErrors).toEqual([])
    expect(pageErrors).toEqual([])

    await app.close()
  })
})
