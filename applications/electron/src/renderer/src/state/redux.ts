export type Action = {
  type: string
}

export type Reducer<S, A extends Action> = (state: S, action: A) => S

export type Dispatch<A extends Action> = (action: A) => void

type Subscriber = () => void

export type ActionListener<S, A extends Action> = (
  action: A,
  before: S,
  after: S,
  dispatch: Dispatch<A>
) => void

export type Store<S, A extends Action> = {
  getState: () => S
  dispatch: Dispatch<A>
  subscribe: (subscriber: Subscriber) => () => void
  listen: (listener: ActionListener<S, A>) => () => void
}

export const createStore = <S, A extends Action>(
  reducer: Reducer<S, A>,
  preloadedState: S
): Store<S, A> => {
  let state = preloadedState
  const subscribers = new Set<Subscriber>()
  const listeners = new Set<ActionListener<S, A>>()

  const getState = (): S => state

  const dispatch: Dispatch<A> = (action) => {
    const before = state
    const after = reducer(before, action)
    if (after !== before) {
      state = after
      for (const subscriber of subscribers) subscriber()
    }
    for (const listener of listeners) listener(action, before, state, dispatch)
  }

  const subscribe = (subscriber: Subscriber): (() => void) => {
    subscribers.add(subscriber)
    return () => {
      subscribers.delete(subscriber)
    }
  }

  const listen = (listener: ActionListener<S, A>): (() => void) => {
    listeners.add(listener)
    return () => {
      listeners.delete(listener)
    }
  }

  return { getState, dispatch, subscribe, listen }
}
