/**
 * Helper to seed verdict-message evidence via /api/test/seed_messages
 * so workflow-gate evidence-check (#1385 / #1386) passes for tests
 * firing accept_story / pass_desk_check. The from-roles map to the
 * required_evidence_roles in the bundled-default workflow.toml.
 */
export const seedTeamVerdicts = async (
  backendPort: number,
  roles: string[],
  text = 'verdict: substrate verified'
): Promise<Response> => {
  const ts = new Date().toISOString()
  return fetch(`http://127.0.0.1:${backendPort}/api/test/seed_messages`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      messages: roles.map((from) => ({ from, text, ts }))
    })
  })
}
