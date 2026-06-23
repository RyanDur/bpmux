import { test, expect } from '@playwright/test'
import { launchIsolatedElectron } from '../helpers/launch'
import { MetricsPage } from '../pages/metricsPage'
import { SparklinePage } from '../pages/sparklinePage'

const daysAgo = (n: number): string => new Date(Date.now() - n * 86_400_000).toISOString()

const fourActivityDaysClimbingThenEasing = [
  { from: 'dev:claude-code', text: '#1 started', ts: daysAgo(4), message_type: 'chat' },
  { from: 'dev:claude-code', text: '#2 started', ts: daysAgo(3), message_type: 'chat' },
  { from: 'dev:claude-code', text: '#3 started', ts: daysAgo(2), message_type: 'chat' },
  { from: 'dev:claude-code', text: '#1 delivered', ts: daysAgo(1), message_type: 'chat' }
]

test.describe('#1237 WIP-count sparkline reflects real lifecycle history', () => {
  test('renders a multi-point trend derived from the actual message log, with no console errors', async () => {
    const { app } = await launchIsolatedElectron({
      seedMessages: fourActivityDaysClimbingThenEasing
    })
    const win = await app.firstWindow()

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
