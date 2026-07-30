import type { ActionListener } from '../redux'
import type { StateSnapshot } from '../reducer'
import type { StateAction } from '../actions'
import { socketDoor } from './door'
import { connection, frames } from './connection'
import { translating } from './translating'
import { chatSend, olderHistory, reactionToggle } from './chat'
import { commentsHydrate, timelineHydrate } from './drillIn'
import {
  addStoryToIteration,
  iterationReorder,
  outcomeHorizon,
  outcomeIterationReorder,
  outcomeReprioritize
} from './board'
import { ptyInput, ptyResize, ptySubscribe } from './pty'

export const socketListeners = (): ActionListener<StateSnapshot, StateAction>[] => {
  const door = socketDoor()
  return [
    connection(door),
    frames(door),
    translating(door, [
      chatSend,
      reactionToggle,
      olderHistory,
      commentsHydrate,
      timelineHydrate,
      outcomeHorizon,
      outcomeIterationReorder,
      outcomeReprioritize,
      iterationReorder,
      addStoryToIteration,
      ptySubscribe,
      ptyInput,
      ptyResize
    ])
  ]
}
