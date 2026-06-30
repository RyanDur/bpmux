export type RpcFn = (tool: string, body: unknown, agent?: string) => Promise<Response>

export const seedTeamVerdicts = async (
  rpc: RpcFn,
  storyId: number,
  roles: string[]
): Promise<void> => {
  for (const role of roles) {
    const resp = await rpc(
      'submit_verdict',
      {
        story_id: storyId,
        verdict: 'verified',
        substrate_grounding: `test-harness: ${role} verdict on #${storyId}`
      },
      role
    )
    if (resp.status !== 200) {
      const text = await resp.text()
      throw new Error(
        `seedTeamVerdicts: submit_verdict for role=${role} on #${storyId} returned ${resp.status} — ${text}`
      )
    }
  }
}
