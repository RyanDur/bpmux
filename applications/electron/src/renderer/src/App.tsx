import { useMemo, useState } from 'react'
import AppShell from './AppShell'
import { useMessageHistory } from './hooks/useMessageHistory'
import { useNotifications } from './hooks/useNotifications'
import { capPerPush, useWebSocket } from './transport/useWebSocket'

const App = (): React.JSX.Element => {
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

  const [atBottom, setAtBottom] = useState<boolean>(true)
  const [pinnedKeys, setPinnedKeys] = useState<ReadonlySet<string>>(() => new Set())

  const { status, messages: liveRaw, sendChat, mirror } = useWebSocket(bridge?.wsUrl, {
    apiBase: bridge?.apiBase,
    authToken: bridge?.authToken
  })

  const liveMessages = useMemo(
    () => capPerPush(liveRaw, atBottom, pinnedKeys),
    [liveRaw, atBottom, pinnedKeys]
  )

  const { combinedMessages, loadOlder, isLoadingOlder, hasMoreOlder, hasError } =
    useMessageHistory({
      liveMessages,
      apiBase: bridge?.apiBase,
      authToken: bridge?.authToken
    })

  useNotifications(combinedMessages)

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
      onPinnedKeysChange={setPinnedKeys}
      ciSnapshot={mirror.ci}
      mirrorReactions={mirror.reactions}
      mirrorPins={mirror.pins}
    />
  )
}

export default App
