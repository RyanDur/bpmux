import './styles/reset.css'
import './styles/tokens.css'
import './styles/shared.css'

import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import App from './App'
import ErrorBoundary from './ErrorBoundary'

// #431: ErrorBoundary wraps <App /> at the renderer root. A render
// throw anywhere below — including inside AppShell itself — surfaces
// as the crash-surface (error type + message + stack + recover +
// copy-details) rather than a black viewport. Substrate for crash
// triage; see `ErrorBoundary.tsx` for the agents-are-users framing.
createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <ErrorBoundary>
      <App />
    </ErrorBoundary>
  </StrictMode>
)
