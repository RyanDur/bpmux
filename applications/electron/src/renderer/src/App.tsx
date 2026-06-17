import { useMemo, useState } from 'react'
import AppShell from './AppShell'
import { useMessageHistory } from './hooks/useMessageHistory'
import { useNotifications } from './hooks/useNotifications'
import { capPerPush, useWebSocket } from './transport/useWebSocket'

const App = (): React.JSX.Element => {
  const bridge = window.bpmux
  if (!bridge) {
    console.error(
      'bpmux: window.bpmux missing — preload did not inject config. ' +
        'Is the main-process agent finished wiring preload?'
    )
  }

  const [atBottom, setAtBottom] = useState<boolean>(true)

  const {
    status,
    messages: liveRaw,
    sendChat,
    mirror,
    boardSnapshot,
    boardInvalidateTick,
    syncStatusTick
  } = useWebSocket(bridge?.wsUrl, {
    apiBase: bridge?.apiBase,
    authToken: bridge?.authToken
  })

  const liveMessages = useMemo(() => capPerPush(liveRaw, atBottom), [liveRaw, atBottom])

  const { combinedMessages, loadOlder, isLoadingOlder, hasMoreOlder, hasError } = useMessageHistory(
    {
      liveMessages,
      apiBase: bridge?.apiBase,
      authToken: bridge?.authToken
    }
  )

  useNotifications()

  return (
    <AppShell
      status={status}
      messages={combinedMessages}
      onSend={sendChat}
      loadOlder={loadOlder}
      isLoadingOlder={isLoadingOlder}
      hasMoreOlder={hasMoreOlder}
      hasError={hasError}
      onAtBottomChange={setAtBottom}
      mirrorRoster={mirror.roster}
      boardSnapshot={boardSnapshot}
      boardInvalidateTick={boardInvalidateTick}
      syncStatusTick={syncStatusTick}
    />
  )
}

export default App
