/**
 * Helper to seed verdicts in the per-session `VerdictsStore` so the
 * workflow-gate evidence-check passes for tests firing accept_story /
 * pass_desk_check. #1407 swapped the data-source from chat-messages
 * to the canonical VerdictsStore — this helper fires
 * `submit_verdict` MCP-calls through the auth-gated `/api/rpc/`
 * surface (same path the bridge uses), one per required role on the
 * given story.
 */
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
