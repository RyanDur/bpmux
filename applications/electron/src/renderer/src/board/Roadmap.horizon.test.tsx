import { cleanup, render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it } from 'vitest'
import Roadmap from './Roadmap'
import { roadmap } from './Roadmap.page'
import { outcome, seedOutcomes } from './Roadmap.fixtures'
import { resetSessionForTest } from '../transport/session/store'
import { startSentFrameCapture, stopSentFrameCapture } from '../transport/session/testCapture'

afterEach(() => {
  cleanup()
  stopSentFrameCapture()
  resetSessionForTest()
})

const bandPick = (outcomeTitle: string, band: string): HTMLElement => {
  const card = roadmap.card(outcomeTitle)
  return within(card).getByRole('button', { name: band })
}

describe('moving an outcome between horizon bands dispatches over the wire', () => {
  it('marks the current band and lets the user pick another', () => {
    seedOutcomes([outcome({ number: 7, title: 'Coming soon', horizon: 'now' })])
    render(<Roadmap />)
    expect(bandPick('Coming soon', 'Now').getAttribute('aria-pressed')).toBe('true')
    expect((bandPick('Coming soon', 'Now') as HTMLButtonElement).disabled).toBe(true)
    expect((bandPick('Coming soon', 'Next') as HTMLButtonElement).disabled).toBe(false)
  })

  it('picking a new band sends a set_outcome_horizon frame for that outcome', async () => {
    seedOutcomes([outcome({ number: 7, title: 'Coming soon', horizon: 'now' })])
    render(<Roadmap />)
    const sent = startSentFrameCapture()
    await userEvent.click(bandPick('Coming soon', 'Next'))
    expect(sent).toEqual([{ type: 'set_outcome_horizon', outcome_number: 7, horizon: 'next' }])
  })

  it('does not change the local band optimistically — the truth returns over the wire', async () => {
    seedOutcomes([outcome({ number: 7, title: 'Coming soon', horizon: 'now' })])
    render(<Roadmap />)
    await userEvent.click(bandPick('Coming soon', 'Later'))
    expect(within(roadmap.band('Now')).queryByText('Coming soon')).toBeTruthy()
    expect(screen.queryByRole('heading', { level: 3, name: 'Later' })).toBeNull()
  })
})
