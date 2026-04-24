import { Component, type ErrorInfo, type ReactNode } from 'react'
import './ErrorBoundary.css'

interface Props {
  children: ReactNode
}

interface State {
  error: Error | null
  componentStack: string | null
  // Secondary-error latch. If the crash-surface itself throws during
  // render, we mark this true and render the minimal fallback so we
  // don't loop the boundary.
  surfaceCrashed: boolean
}

// #431: top-level ErrorBoundary for the Electron renderer.
//
// Before: any render-time or lifecycle throw tore down the React tree
// silently and left a black viewport. Owner-reported crashes had no
// diagnostic signal unless DevTools happened to be open pre-crash.
//
// After: renderer root is wrapped in this boundary. A throw produces
// a visible surface with error type + message + component stack + a
// Try-again / Copy-details affordance. The owner can report the error
// + recover without restarting the app.
//
// Agents-are-users framing: dev (triaging owner-reports), designer
// (during desk-check), pm (facilitating acceptance), and Ryan (primary
// owner) all consume this surface when crashes happen. Treating it
// as load-bearing substrate, not polish.

class ErrorBoundary extends Component<Props, State> {
  state: State = {
    error: null,
    componentStack: null,
    surfaceCrashed: false
  }

  static getDerivedStateFromError(error: Error): Partial<State> {
    // Fires during render on the first throw — React commits this
    // state update before unmounting the failed subtree. Capturing
    // `error` here ensures the boundary re-renders with the surface
    // visible on the same commit.
    return { error }
  }

  componentDidCatch(error: Error, info: ErrorInfo): void {
    // Fires after the boundary commits its fallback. We store the
    // component stack separately because `getDerivedStateFromError`
    // only receives the Error object — the componentStack lives on
    // ErrorInfo and is crucial for locating *where* the throw came
    // from (useful to Ryan when pasting the error into a bug report).
    this.setState({ componentStack: info.componentStack ?? null })
    // Also log to the renderer console so DevTools captures it when
    // opened post-crash.
    console.error('[bpmux ErrorBoundary]', error, info.componentStack)
  }

  handleReset = (): void => {
    this.setState({ error: null, componentStack: null, surfaceCrashed: false })
  }

  handleCopy = async (): Promise<void> => {
    const payload = {
      type: this.state.error?.name ?? 'Error',
      message: this.state.error?.message ?? '',
      stack: this.state.error?.stack ?? '',
      componentStack: this.state.componentStack ?? '',
      userAgent: typeof navigator !== 'undefined' ? navigator.userAgent : '',
      timestamp: new Date().toISOString()
    }
    try {
      await navigator.clipboard.writeText(JSON.stringify(payload, null, 2))
    } catch {
      // Clipboard API failure is non-fatal — surface stays rendered;
      // Ryan can select + copy manually from the <pre> text. Don't
      // re-throw (would poison the already-crashed state).
    }
  }

  render(): ReactNode {
    const { error, componentStack, surfaceCrashed } = this.state
    if (!error) return this.props.children

    // Secondary-error fallback: if rendering the surface itself threw
    // (caught via a manual try/catch wrapper around the surface JSX),
    // we render a minimal <pre> so the boundary can't loop.
    if (surfaceCrashed) {
      return (
        <pre className="error-boundary-fallback">
          {String(error.name)}: {String(error.message)}
        </pre>
      )
    }

    try {
      return (
        <section className="error-boundary-surface" role="alert" aria-live="assertive">
          <header className="error-boundary-header">
            <strong className="error-boundary-title">{error.name ?? 'Error'}</strong>
            <p className="error-boundary-message">{error.message}</p>
          </header>
          {componentStack && (
            <details className="error-boundary-details">
              <summary>component stack</summary>
              <pre>{componentStack}</pre>
            </details>
          )}
          {error.stack && (
            <details className="error-boundary-details">
              <summary>error stack</summary>
              <pre>{error.stack}</pre>
            </details>
          )}
          <footer className="error-boundary-actions">
            <button type="button" onClick={this.handleReset}>
              Try again
            </button>
            <button type="button" onClick={this.handleCopy}>
              Copy details
            </button>
          </footer>
        </section>
      )
    } catch (secondary) {
      // If the surface JSX throws during its own render (e.g., a
      // broken token reference), flip the latch + re-render with the
      // minimal fallback. Log both errors so the reader can see the
      // cause-chain.
      console.error('[bpmux ErrorBoundary] secondary error in surface', secondary)
      // Note: setState inside render is a React anti-pattern, but
      // `queueMicrotask` schedules it for after commit. We render
      // fallback synchronously from the catch to avoid a frame of
      // blank-screen between the throws.
      queueMicrotask(() => this.setState({ surfaceCrashed: true }))
      return (
        <pre className="error-boundary-fallback">
          {String(error.name)}: {String(error.message)}
        </pre>
      )
    }
  }
}

export default ErrorBoundary
