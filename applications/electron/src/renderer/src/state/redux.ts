export interface Action {
  type: string
}

export type Reducer<S, A extends Action> = (state: S, action: A) => S

export type Dispatch<A extends Action> = (action: A) => void

type Listener = () => void

export interface Store<S, A extends Action> {
  getState: () => S
  dispatch: Dispatch<A>
  subscribe: (listener: Listener) => () => void
}

export interface MiddlewareApi<S, A extends Action> {
  getState: () => S
  dispatch: Dispatch<A>
}

export type Middleware<S, A extends Action> = (
  api: MiddlewareApi<S, A>
) => (next: Dispatch<A>) => Dispatch<A>

export const applyMiddleware =
  <S, A extends Action>(...middlewares: Middleware<S, A>[]) =>
  (base: Dispatch<A>, api: MiddlewareApi<S, A>): Dispatch<A> => {
    const chain = middlewares.map((middleware) => middleware(api))
    return chain.reduceRight((next, middleware) => middleware(next), base)
  }

export const createStore = <S, A extends Action>(
  reducer: Reducer<S, A>,
  preloadedState: S,
  enhancer?: (base: Dispatch<A>, api: MiddlewareApi<S, A>) => Dispatch<A>
): Store<S, A> => {
  let state = preloadedState
  const listeners = new Set<Listener>()

  const getState = (): S => state

  const baseDispatch: Dispatch<A> = (action) => {
    const next = reducer(state, action)
    if (next === state) return
    state = next
    for (const listener of listeners) listener()
  }

  const subscribe = (listener: Listener): (() => void) => {
    listeners.add(listener)
    return () => {
      listeners.delete(listener)
    }
  }

  const dispatch: Dispatch<A> = enhancer
    ? enhancer(baseDispatch, { getState, dispatch: (action) => dispatch(action) })
    : baseDispatch

  return { getState, dispatch, subscribe }
}
