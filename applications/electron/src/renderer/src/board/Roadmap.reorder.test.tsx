import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it } from 'vitest'
import Roadmap from './Roadmap'
import { buildingIteration, card, checks, outcome, seedRoadmap } from './Roadmap.fixtures'
import { resetStateForTest } from '../transport/state/store'
import { startSentFrameCapture, stopSentFrameCapture } from '../transport/state/testCapture'

afterEach(() => {
  cleanup()
  stopSentFrameCapture()
  resetStateForTest()
})

const dragTransfer = (): DataTransfer => {
  const stored: Record<string, string> = {}
  const types: string[] = []
  return {
    setData: (key: string, value: string) => {
      stored[key] = value
      if (!types.includes(key)) types.push(key)
    },
    getData: (key: string): string => stored[key] ?? '',
    setDragImage: () => {},
    effectAllowed: 'none',
    dropEffect: 'none',
    types
  } as unknown as DataTransfer
}

const seedTwo = (): void =>
  seedRoadmap(
    [card({ id: 5, name: 'Outcome five', horizon: 'now' })],
    [
      outcome({
        id: 5,
        title: 'Outcome five',
        horizon: 'now',
        iterations: [
          buildingIteration(710, 'Crux', 1, checks(true, true, false, false)),
          buildingIteration(720, 'Lyra', 2, checks(false, false, false, false))
        ]
      })
    ]
  )

describe('dragging an iteration in the drill-in dispatches a reorder over the wire', () => {
  it('drops the dragged iteration onto a sibling and sends its new 1-indexed position', async () => {
    seedTwo()
    render(<Roadmap />)
    await userEvent.click(screen.getByRole('button', { name: 'Outcome five' }))

    const sent = startSentFrameCapture()
    const transfer = dragTransfer()
    fireEvent.dragStart(screen.getByRole('button', { name: 'Drag to reorder #710' }), {
      dataTransfer: transfer
    })
    fireEvent.drop(screen.getByRole('button', { name: 'Drag to reorder #720' }), {
      dataTransfer: transfer
    })

    expect(sent).toEqual([
      { type: 'reorder_outcome_iteration', outcome_id: 5, iteration_id: 710, position: 2 }
    ])
  })
})
