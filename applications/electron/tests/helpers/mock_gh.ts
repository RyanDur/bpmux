// #1127 sister-class helper: TypeScript port of Rust harness/mock_gh.rs
// (#724 slice 2). Builds GitHub API JSON shapes that octocrab's
// deserializer accepts + a tiny http.Server harness so Playwright
// specs can drive bpmux's real GitHubSourceAdapter against an
// in-process mock GH boundary.
//
// Per Ryan 2026-04-30: only octocrab gets test-doubled. The mock
// serves the GH HTTP boundary; everything inside bpmux runs the
// production code path.
import { createServer, type IncomingMessage, type ServerResponse } from 'http'
import type { AddressInfo } from 'net'

// Octocrab's Label deserializer rejects skinny `{name}` payloads —
// the entire issue gets silently dropped by serde, leaving the
// BoardStore empty with no error surface. label_json mirrors the
// Rust helper: emit the full shape from a single name argument.
export function labelJson(name: string): Record<string, unknown> {
  let id = 0
  for (const c of Buffer.from(name, 'utf-8')) {
    id = (id * 131 + c) >>> 0
  }
  return {
    id,
    node_id: `MDU6TGFiZWw${id}`,
    url: `https://example.test/labels/${name}`,
    name,
    description: null,
    color: 'ededed',
    default: false
  }
}

// Octocrab's Comment deserializer needs more than `{id}` — without
// the full shape it silently fails on drain-push, manifesting as a
// hard-to-debug log_diag warning instead of a test failure.
export function commentJson(body: string): Record<string, unknown> {
  return {
    id: 1,
    node_id: 'MDEyOklzc3VlQ29tbWVudDE=',
    url: 'https://example.test/comments/1',
    html_url: 'https://example.test/web/comment/1',
    body,
    author_association: 'OWNER',
    user: ghUser(),
    created_at: '2026-04-30T00:00:00Z'
  }
}

export function issueJson(
  number: number,
  title: string,
  labels: ReadonlyArray<string>,
  body: string | null
): Record<string, unknown> {
  return {
    id: number,
    node_id: 'MDU6SXNzdWUx',
    url: `https://example.test/issues/${number}`,
    repository_url: 'https://example.test/repo',
    labels_url: `https://example.test/issues/${number}/labels{/name}`,
    comments_url: `https://example.test/issues/${number}/comments`,
    events_url: `https://example.test/issues/${number}/events`,
    html_url: `https://example.test/web/${number}`,
    number,
    state: 'open',
    state_reason: null,
    title,
    body,
    user: ghUser(),
    labels: labels.map(labelJson),
    assignees: [],
    author_association: 'OWNER',
    comments: 0,
    created_at: '2026-04-26T19:30:00Z',
    updated_at: '2026-04-26T19:30:00Z',
    closed_at: null,
    milestone: null,
    pull_request: null,
    active_lock_reason: null,
    locked: false
  }
}

function ghUser(): Record<string, unknown> {
  return {
    login: 'actor',
    id: 1,
    node_id: 'MDQ6VXNlcjE=',
    avatar_url: 'https://example.test/avatar.png',
    gravatar_id: '',
    url: 'https://example.test/users/actor',
    html_url: 'https://example.test/actor',
    followers_url: 'https://example.test/actor/followers',
    following_url: 'https://example.test/actor/following{/other_user}',
    gists_url: 'https://example.test/actor/gists{/gist_id}',
    starred_url: 'https://example.test/actor/starred{/owner}{/repo}',
    subscriptions_url: 'https://example.test/actor/subscriptions',
    organizations_url: 'https://example.test/actor/orgs',
    repos_url: 'https://example.test/actor/repos',
    events_url: 'https://example.test/actor/events{/privacy}',
    received_events_url: 'https://example.test/actor/received_events',
    type: 'User',
    site_admin: false
  }
}

export interface MockRoute {
  method: 'GET' | 'POST' | 'PATCH' | 'DELETE'
  // Exact path prefix or regex. Matched in registration order.
  match: string | RegExp
  status?: number
  bodyJson?: unknown
}

export interface MockGhServer {
  baseUrl: string
  port: number
  // Register an additional route. Routes registered later take
  // precedence over earlier ones with the same path (last-write-wins).
  on(route: MockRoute): void
  // Mutable in-memory state for tests that need to grow the issue
  // list across mutations (e.g., create then list then re-fetch).
  setIssues(issues: ReadonlyArray<Record<string, unknown>>): void
  close(): Promise<void>
}

// Spawn an in-process http.Server that serves canned GH responses.
// Default route set covers the endpoints handle_* hits during the
// gate-tool flow (create / start / deliver / accept / fail / close):
//   GET    /repos/acme/bpmux/issues          → current `issues` array
//   POST   /repos/acme/bpmux/issues          → append to `issues`, return new
//   GET    /repos/acme/bpmux/issues/{n}      → matching issue or 404
//   PATCH  /repos/acme/bpmux/issues/{n}      → echo issue
//   POST   /repos/acme/bpmux/issues/{n}/labels   → []
//   DELETE /repos/acme/bpmux/issues/{n}/labels/* → []
//   POST   /repos/acme/bpmux/issues/{n}/comments → comment_json("ok")
// Tests can override via on() before the gate-tool fires.
export async function startMockGhServer(): Promise<MockGhServer> {
  const issues: Record<string, unknown>[] = []
  const overrides: MockRoute[] = []

  const server = createServer((req: IncomingMessage, res: ServerResponse) => {
    const url = req.url ?? ''
    const method = (req.method ?? 'GET') as MockRoute['method']

    let body = ''
    req.on('data', (chunk: Buffer) => {
      body += chunk.toString('utf-8')
    })
    req.on('end', () => {
      // Override match wins (last-registered first).
      for (let i = overrides.length - 1; i >= 0; i--) {
        const route = overrides[i]
        if (route.method !== method) continue
        const matches =
          typeof route.match === 'string' ? url === route.match : route.match.test(url)
        if (matches) {
          send(res, route.status ?? 200, route.bodyJson ?? null)
          return
        }
      }

      // Defaults.
      const issuesPath = '/repos/acme/bpmux/issues'
      if (method === 'GET' && url.startsWith(`${issuesPath}?`)) {
        send(res, 200, issues)
        return
      }
      if (method === 'GET' && url === issuesPath) {
        send(res, 200, issues)
        return
      }
      if (method === 'POST' && url === issuesPath) {
        // Body has title + body; assign next number, default labels=[icebox].
        const parsed = body ? (JSON.parse(body) as Record<string, unknown>) : {}
        const nextNumber = issues.length + 1
        const issue = issueJson(
          nextNumber,
          (parsed.title as string) ?? `[Feature] mock-${nextNumber}`,
          ['icebox'],
          (parsed.body as string) ?? null
        )
        issues.push(issue)
        send(res, 201, issue)
        return
      }
      const issueDetail = url.match(/^\/repos\/acme\/bpmux\/issues\/(\d+)(?:\?.*)?$/)
      if (issueDetail) {
        const n = Number(issueDetail[1])
        const found = issues.find((i) => i['number'] === n)
        if (method === 'GET') {
          if (found) {
            send(res, 200, found)
          } else {
            send(res, 404, { message: 'Not Found' })
          }
          return
        }
        if (method === 'PATCH') {
          // Merge incoming PATCH body into stored issue. close_story
          // PATCHes `{state: "closed"}` via octocrab; without the
          // merge, refresh_story parses the issue's `state` as "open"
          // forever and the renderer never sees the closed transition.
          const target = found as
            | (Record<string, unknown> & { state?: string; labels?: unknown[] })
            | undefined
          if (target) {
            const patch = body ? (JSON.parse(body) as Record<string, unknown>) : {}
            for (const [k, v] of Object.entries(patch)) {
              if (k === 'labels' && Array.isArray(v)) {
                target.labels = (v as string[]).map(labelJson)
              } else {
                target[k] = v
              }
            }
            send(res, 200, target)
          } else {
            send(res, 200, issueJson(n, 'mock', [], null))
          }
          return
        }
      }
      // Label add: parse body's `labels: [name, ...]` and merge into
      // the issue's stored `labels`. octocrab returns the post-update
      // label set; refresh_story re-parses the issue's labels to
      // derive `state`, so persisting the additions is what makes
      // start/deliver/pass/accept transitions visible to the next
      // refresh (real GH does this; mock had been silently dropping
      // mutations and returning the original label set).
      const addLabels = url.match(/^\/repos\/acme\/bpmux\/issues\/(\d+)\/labels(?:\?.*)?$/)
      if (method === 'POST' && addLabels) {
        const n = Number(addLabels[1])
        const target = issues.find((i) => i['number'] === n) as
          | { labels?: Array<{ name?: string }>; title?: string; body?: string | null }
          | undefined
        if (target) {
          const parsed = body ? (JSON.parse(body) as { labels?: string[] }) : {}
          const adds = parsed.labels ?? []
          const existing = (target.labels ?? []).map((l) => l.name ?? '').filter(Boolean)
          const merged = Array.from(new Set([...existing, ...adds]))
          target.labels = merged.map(labelJson)
          send(res, 200, target.labels)
          return
        }
        send(res, 200, [])
        return
      }
      const removeLabel = url.match(
        /^\/repos\/acme\/bpmux\/issues\/(\d+)\/labels\/(.+?)(?:\?.*)?$/
      )
      if (method === 'DELETE' && removeLabel) {
        const n = Number(removeLabel[1])
        const labelName = decodeURIComponent(removeLabel[2])
        const target = issues.find((i) => i['number'] === n) as
          | { labels?: Array<{ name?: string }> }
          | undefined
        if (target) {
          target.labels = (target.labels ?? []).filter(
            (l) => (l.name ?? '').toLowerCase() !== labelName.toLowerCase()
          )
          send(res, 200, target.labels)
          return
        }
        send(res, 200, [])
        return
      }
      if (
        method === 'POST' &&
        /^\/repos\/acme\/bpmux\/issues\/\d+\/comments(\?.*)?$/.test(url)
      ) {
        send(res, 201, commentJson('ok'))
        return
      }
      // ci_watcher polls this every few seconds. Default empty
      // workflow_runs so the renderer's /api/ci endpoint resolves
      // cleanly (no console errors from a 404→502 cascade).
      if (
        method === 'GET' &&
        /^\/repos\/acme\/bpmux\/actions\/runs(\?.*)?$/.test(url)
      ) {
        send(res, 200, { total_count: 0, workflow_runs: [] })
        return
      }
      // Drill-in opens fetch comments + timeline. Default empty so
      // open-on-row-click doesn't 502.
      if (
        method === 'GET' &&
        /^\/repos\/acme\/bpmux\/issues\/\d+\/comments(\?.*)?$/.test(url)
      ) {
        send(res, 200, [])
        return
      }
      if (
        method === 'GET' &&
        /^\/repos\/acme\/bpmux\/issues\/\d+\/timeline(\?.*)?$/.test(url)
      ) {
        send(res, 200, [])
        return
      }
      // Fallback: 404 with a body that won't crash octocrab if it
      // tries to deserialize. Tests should add explicit overrides for
      // any missing route; an unmatched call is a test-setup smell.
      send(res, 404, { message: `mock-gh: no route for ${method} ${url}` })
    })
  })

  await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', resolve))
  const port = (server.address() as AddressInfo).port

  return {
    baseUrl: `http://127.0.0.1:${port}/`,
    port,
    on(route) {
      overrides.push(route)
    },
    setIssues(seed) {
      issues.length = 0
      issues.push(...seed)
    },
    async close() {
      await new Promise<void>((resolve, reject) => {
        server.close((err?: Error) => (err ? reject(err) : resolve()))
      })
    }
  }
}

function send(res: ServerResponse, status: number, body: unknown): void {
  res.statusCode = status
  res.setHeader('content-type', 'application/json')
  res.end(JSON.stringify(body))
}
