import type { Middleware } from '../redux'
import type { StateSnapshot } from '../reducer'
import type { StateAction } from '../actions'
import { socketDoor } from './door'
import { connection, frames } from './connection'
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

export const socketMiddlewares = (): Middleware<StateSnapshot, StateAction>[] => {
  const door = socketDoor()
  return [
    connection(door),
    frames(door),
    chatSend(door),
    reactionToggle(door),
    olderHistory(door),
    commentsHydrate(door),
    timelineHydrate(door),
    outcomeHorizon(door),
    outcomeIterationReorder(door),
    outcomeReprioritize(door),
    iterationReorder(door),
    addStoryToIteration(door),
    ptySubscribe(door),
    ptyInput(door),
    ptyResize(door)
  ]
}
