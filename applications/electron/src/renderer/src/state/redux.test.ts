import { describe, expect, it, vi } from 'vitest'
import { applyMiddleware, createStore, type Middleware } from './redux'

type CountState = {
  count: number
}

type CountAction = { type: 'inc' } | { type: 'add'; by: number } | { type: 'noop' }

const reducer = (state: CountState, action: CountAction): CountState => {
  switch (action.type) {
    case 'inc':
      return { count: state.count + 1 }
    case 'add':
      return { count: state.count + action.by }
    default:
      return state
  }
}

describe('createStore', () => {
  it('runs the reducer on dispatch and reflects it in getState', () => {
    const store = createStore(reducer, { count: 0 })
    store.dispatch({ type: 'inc' })
    store.dispatch({ type: 'add', by: 5 })
    expect(store.getState()).toEqual({ count: 6 })
  })

  it('notifies subscribers on a state change and stops after unsubscribe', () => {
    const store = createStore(reducer, { count: 0 })
    const listener = vi.fn()
    const unsubscribe = store.subscribe(listener)
    store.dispatch({ type: 'inc' })
    expect(listener).toHaveBeenCalledTimes(1)
    unsubscribe()
    store.dispatch({ type: 'inc' })
    expect(listener).toHaveBeenCalledTimes(1)
  })

  it('does NOT notify when the reducer returns the same state reference (no-op action)', () => {
    const store = createStore(reducer, { count: 0 })
    const listener = vi.fn()
    store.subscribe(listener)
    store.dispatch({ type: 'noop' })
    expect(listener).not.toHaveBeenCalled()
    expect(store.getState()).toEqual({ count: 0 })
  })
})

describe('applyMiddleware', () => {
  it('lets middleware observe every dispatched action and pass it through next', () => {
    const seen: string[] = []
    const logger: Middleware<CountState, CountAction> = () => (next) => (action) => {
      seen.push(action.type)
      next(action)
    }
    const store = createStore(reducer, { count: 0 }, applyMiddleware(logger))
    store.dispatch({ type: 'inc' })
    expect(seen).toEqual(['inc'])
    expect(store.getState()).toEqual({ count: 1 })
  })

  it('composes middleware right-to-left (outer wraps inner)', () => {
    const order: string[] = []
    const outer: Middleware<CountState, CountAction> = () => (next) => (action) => {
      order.push('outer:before')
      next(action)
      order.push('outer:after')
    }
    const inner: Middleware<CountState, CountAction> = () => (next) => (action) => {
      order.push('inner:before')
      next(action)
      order.push('inner:after')
    }
    const store = createStore(reducer, { count: 0 }, applyMiddleware(outer, inner))
    store.dispatch({ type: 'inc' })
    expect(order).toEqual(['outer:before', 'inner:before', 'inner:after', 'outer:after'])
  })

  it('routes middleware-initiated dispatch through the full chain', () => {
    const seen: string[] = []
    const doubler: Middleware<CountState, CountAction> = (api) => (next) => (action) => {
      seen.push(action.type)
      next(action)
      if (action.type === 'inc') api.dispatch({ type: 'add', by: 10 })
    }
    const store = createStore(reducer, { count: 0 }, applyMiddleware(doubler))
    store.dispatch({ type: 'inc' })
    expect(seen).toEqual(['inc', 'add'])
    expect(store.getState()).toEqual({ count: 11 })
  })
})
