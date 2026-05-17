/**
 * Labels mutation client (#1352). Replaces raw fetch() at the
 * presentation/transport boundary so Iteration/Board components
 * don't carry HTTP-shape constants (Content-Type, Authorization)
 * inline.
 *
 * Returns a tagged-result so callers branch on `result.ok` without
 * try-catch around an HTTP boundary — sister-shape to the
 * `bridge::view_model` Rust-side anti-corruption layer.
 */

export interface LabelMutation {
  add: string[]
  remove: string[]
}

export interface ApiContext {
  apiBase: string
  authToken: string | null | undefined
}

export interface ClientError {
  kind: 'http' | 'network'
  status?: number
  message: string
}

export type ClientResult<T = void> =
  | { ok: true; value: T }
  | { ok: false; error: ClientError }

export async function postStoryLabels(
  ctx: ApiContext,
  storyNumber: number,
  mutation: LabelMutation
): Promise<ClientResult> {
  try {
    const res = await fetch(`${ctx.apiBase}/api/stories/${storyNumber}/labels`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${ctx.authToken ?? ''}`
      },
      body: JSON.stringify(mutation)
    })
    if (!res.ok) {
      return {
        ok: false,
        error: { kind: 'http', status: res.status, message: `HTTP ${res.status}` }
      }
    }
    return { ok: true, value: undefined }
  } catch (err) {
    return {
      ok: false,
      error: {
        kind: 'network',
        message: err instanceof Error ? err.message : String(err)
      }
    }
  }
}
