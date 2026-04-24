import { Component, type ErrorInfo, type ReactNode } from 'react'
import './ErrorBoundary.css'

interface Props {
  children: ReactNode
}

interface State {
  error: Error | null
  componentStack: string | null
  surfaceCrashed: boolean
}

class ErrorBoundary extends Component<Props, State> {
  state: State = {
    error: null,
    componentStack: null,
    surfaceCrashed: false
  }

  static getDerivedStateFromError(error: Error): Partial<State> {
    return { error }
  }

  componentDidCatch(error: Error, info: ErrorInfo): void {
    this.setState({ componentStack: info.componentStack ?? null })
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
      return
    }
  }

  render(): ReactNode {
    const { error, componentStack, surfaceCrashed } = this.state
    if (!error) return this.props.children

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
      console.error('[bpmux ErrorBoundary] secondary error in surface', secondary)
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
