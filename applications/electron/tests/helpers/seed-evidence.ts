export type RpcFn = (
  tool: string,
  body: unknown,
  agent?: string
) => Promise<Response>

export const seedTeamVerdicts = async (
  rpc: RpcFn,
  storyNumber: number,
  roles: string[]
): Promise<void> => {
  for (const role of roles) {
    const resp = await rpc(
      'submit_verdict',
      {
        story_number: storyNumber,
        verdict: 'verified',
        substrate_grounding: `test-harness: ${role} verdict on #${storyNumber}`
      },
      role
    )
    if (resp.status !== 200) {
      const text = await resp.text()
      throw new Error(
        `seedTeamVerdicts: submit_verdict for role=${role} on #${storyNumber} returned ${resp.status} — ${text}`
      )
    }
  }
}
