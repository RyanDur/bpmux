import { useState, type ReactNode } from 'react'
import { ErrorBoundary as LibErrorBoundary, type FallbackProps } from 'react-error-boundary'
import { asyncTryCatch, toError } from '@ryandur/sand'
import './ErrorBoundary.css'

type Props = {
  children: ReactNode
}

type ErrorShape = {
  name: string
  message: string
  stack: string | null
}

const toErrorShape = (thrown: unknown): ErrorShape => {
  if (thrown instanceof Error) {
    return { name: thrown.name, message: thrown.message, stack: thrown.stack ?? null }
  }
  return { name: 'Error', message: String(thrown), stack: null }
}

const copyErrorDetails = async (
  error: ErrorShape,
  componentStack: string | null
): Promise<void> => {
  const payload = {
    type: error.name,
    message: error.message,
    stack: error.stack ?? '',
    componentStack: componentStack ?? '',
    userAgent: typeof navigator !== 'undefined' ? navigator.userAgent : '',
    timestamp: new Date().toISOString()
  }
  await asyncTryCatch(
    () => navigator.clipboard.writeText(JSON.stringify(payload, null, 2)),
    toError
  ).value
}

const PlainFallback = ({ error }: FallbackProps): React.JSX.Element => {
  const shape = toErrorShape(error)
  return (
    <pre className="error-boundary-fallback">
      {shape.name}: {shape.message}
    </pre>
  )
}

type SurfaceProps = {
  error: unknown
  componentStack: string | null
  resetErrorBoundary: () => void
}

const Surface = ({
  error,
  componentStack,
  resetErrorBoundary
}: SurfaceProps): React.JSX.Element => {
  const shape = toErrorShape(error)
  return (
    <section className="error-boundary-surface" role="alert" aria-live="assertive">
      <header className="error-boundary-header">
        <strong className="error-boundary-title">{shape.name}</strong>
        <p className="error-boundary-message">{shape.message}</p>
      </header>
      {componentStack && (
        <details className="error-boundary-details">
          <summary>component stack</summary>
          <pre>{componentStack}</pre>
        </details>
      )}
      {shape.stack && (
        <details className="error-boundary-details">
          <summary>error stack</summary>
          <pre>{shape.stack}</pre>
        </details>
      )}
      <footer className="error-boundary-actions">
        <button type="button" onClick={resetErrorBoundary}>
          Try again
        </button>
        <button type="button" onClick={() => void copyErrorDetails(shape, componentStack)}>
          Copy details
        </button>
      </footer>
    </section>
  )
}

const ErrorBoundary = ({ children }: Props): React.JSX.Element => {
  const [componentStack, setComponentStack] = useState<string | null>(null)
  return (
    <LibErrorBoundary
      onError={(error, info) => {
        setComponentStack(info.componentStack ?? null)
        console.error('[bpmux ErrorBoundary]', error, info.componentStack)
      }}
      onReset={() => setComponentStack(null)}
      fallbackRender={({ error, resetErrorBoundary }) => (
        <LibErrorBoundary FallbackComponent={PlainFallback}>
          <Surface
            error={error}
            componentStack={componentStack}
            resetErrorBoundary={resetErrorBoundary}
          />
        </LibErrorBoundary>
      )}
    >
      {children}
    </LibErrorBoundary>
  )
}

export default ErrorBoundary
