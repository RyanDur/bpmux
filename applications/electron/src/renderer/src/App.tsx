import { useEffect, useMemo, useState } from 'react'
import AppShell from './AppShell'
import { useMessageHistory } from './hooks/useMessageHistory'
import { useNotifications } from './hooks/useNotifications'
import { capPerPush } from './transport/messageCap'
import { sendChat, startConnection, useConnectionStatus, useLiveMessages } from './transport/state'

const App = (): React.JSX.Element => {
  const bridge = window.bpmux
  if (!bridge) {
    console.error(
      'bpmux: window.bpmux missing — preload did not inject config. ' +
        'Is the main-process agent finished wiring preload?'
    )
  }

  const [atBottom, setAtBottom] = useState<boolean>(true)

  useEffect(() => {
    startConnection({
      url: bridge?.wsUrl,
      apiBase: bridge?.apiBase,
      authToken: bridge?.authToken,
      ownerNonce: bridge?.ownerNonce
    })
  }, [bridge?.wsUrl, bridge?.apiBase, bridge?.authToken, bridge?.ownerNonce])

  const status = useConnectionStatus()
  const liveRaw = useLiveMessages()
  const liveMessages = useMemo(() => capPerPush(liveRaw, atBottom), [liveRaw, atBottom])

  const { combinedMessages, loadOlder, isLoadingOlder, hasMoreOlder, hasError } = useMessageHistory(
    {
      liveMessages,
      apiBase: bridge?.apiBase
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
    />
  )
}

export default App
