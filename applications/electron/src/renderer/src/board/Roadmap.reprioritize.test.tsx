import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it } from 'vitest'
import Roadmap from './Roadmap'
import { outcome, seedOutcomes } from './Roadmap.fixtures'
import { resetSessionForTest } from '../transport/session/store'
import { startSentFrameCapture, stopSentFrameCapture } from '../transport/session/testCapture'

afterEach(() => {
  cleanup()
  stopSentFrameCapture()
  resetSessionForTest()
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

describe('dragging an outcome within a band dispatches a reprioritize over the wire', () => {
  it('drops the dragged outcome onto a peer and sends its new 1-indexed position', () => {
    seedOutcomes([
      outcome({ number: 10, title: 'Leads', horizon: 'now', horizon_order: 1 }),
      outcome({ number: 20, title: 'Trails', horizon: 'now', horizon_order: 2 })
    ])
    render(<Roadmap />)

    const sent = startSentFrameCapture()
    const transfer = dragTransfer()
    fireEvent.dragStart(screen.getByRole('button', { name: 'Drag to reorder #10' }), {
      dataTransfer: transfer
    })
    fireEvent.drop(screen.getByRole('button', { name: 'Drag to reorder #20' }), {
      dataTransfer: transfer
    })

    expect(sent).toEqual([{ type: 'reprioritize_outcome', outcome_number: 10, position: 2 }])
  })
})
