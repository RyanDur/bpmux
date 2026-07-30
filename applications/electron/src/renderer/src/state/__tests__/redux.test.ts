import { describe, expect, it, vi } from 'vitest'
import { createStore } from '../redux'

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

describe('action listeners — the renderer mirrors the server engine: store = reducer + listeners', () => {
  it('a listener observes every action after the reduce, even when state does not change', () => {
    const seen: string[] = []
    const store = createStore(reducer, { count: 0 })
    store.listen((action, _before, after) => seen.push(`${action.type}:${after.count}`))
    store.dispatch({ type: 'inc' })
    store.dispatch({ type: 'noop' })
    expect(seen).toEqual(['inc:1', 'noop:1'])
  })

  it('a listener sees both sides of the reduce', () => {
    const store = createStore(reducer, { count: 0 })
    let observed: string | null = null
    store.listen((action, before, after) => {
      if (action.type === 'inc') observed = `${before.count}->${after.count}`
    })
    store.dispatch({ type: 'inc' })
    expect(observed).toEqual('0->1')
  })

  it('a listener-initiated dispatch runs the full cycle, listeners included', () => {
    const seen: string[] = []
    const store = createStore(reducer, { count: 0 })
    store.listen((action, _before, _after, dispatch) => {
      seen.push(action.type)
      if (action.type === 'inc') dispatch({ type: 'add', by: 10 })
    })
    store.dispatch({ type: 'inc' })
    expect(seen).toEqual(['inc', 'add'])
    expect(store.getState()).toEqual({ count: 11 })
  })

  it('listeners run in registration order and can be removed', () => {
    const order: string[] = []
    const store = createStore(reducer, { count: 0 })
    const forget = store.listen(() => order.push('first'))
    store.listen(() => order.push('second'))
    store.dispatch({ type: 'inc' })
    forget()
    store.dispatch({ type: 'inc' })
    expect(order).toEqual(['first', 'second', 'second'])
  })
})
