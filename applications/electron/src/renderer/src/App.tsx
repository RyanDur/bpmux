import BoardRegion from './components/BoardRegion'
import ChatRegion from './components/ChatRegion'
import Sidebar from './components/Sidebar'
import Titlebar from './components/Titlebar'
import { useWebSocket } from './useWebSocket'

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

  const { status, messages, sendBroadcast } = useWebSocket(bridge?.wsUrl)

  return (
    <div className="app" data-test="app-root">
      <Titlebar status={status} />
      <div className="app-body">
        <Sidebar />
        <ChatRegion messages={messages} status={status} onSend={sendBroadcast} />
        <BoardRegion />
      </div>
    </div>
  )
}

export default App
