import AppShell from './AppShell'
import { useWebSocket } from './transport/useWebSocket'

function App(): React.JSX.Element {
  // `window.bpmux` is injected by the preload script owned by the
  // parallel main-process agent. If it's missing (e.g., renderer
  // loaded without preload during dev), we still render the shell
  // and surface the error in devtools — no white screen.
  const bridge = window.bpmux
  if (!bridge) {
    console.error(
      'bpmux: window.bpmux missing — preload did not inject config. ' +
        'Is the main-process agent finished wiring preload?'
    )
  }

  const { status, messages, sendChat } = useWebSocket(
    bridge?.wsUrl,
    bridge?.apiBase,
    bridge?.authToken
  )

  return <AppShell status={status} messages={messages} onSend={sendChat} />
}

export default App
