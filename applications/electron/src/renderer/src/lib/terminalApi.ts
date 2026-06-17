type Bridge = Window['bpmux']

export const spawnTerminalRequest = (bridge: Bridge): Promise<Response> =>
  fetch(`${bridge?.apiBase}/api/terminals/spawn`, {
    method: 'POST',
    headers: { authorization: `Bearer ${bridge?.authToken}` }
  })

export const closeTerminalRequest = (
  bridge: Bridge,
  id: string,
  force: boolean
): Promise<Response> =>
  fetch(`${bridge?.apiBase}/api/terminals/close`, {
    method: 'POST',
    headers: {
      authorization: `Bearer ${bridge?.authToken}`,
      'content-type': 'application/json'
    },
    body: JSON.stringify({ terminal_id: id, force })
  })

export const listTerminalsRequest = (bridge: Bridge): Promise<Response> =>
  fetch(`${bridge?.apiBase}/api/terminals`, {
    headers: { authorization: `Bearer ${bridge?.authToken}` }
  })
