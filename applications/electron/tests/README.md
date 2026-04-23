# Playwright desk-check harness

## Mandatory: use the isolated launch helper

Desk-check specs MUST spawn Electron through
`tests/helpers/launch.ts`. Direct `electron.launch(...)` calls in
session-spawning tests are prohibited — they skip port / session /
user-data isolation and can collide with the dev's live bpmux on
`4117`.

```ts
import { launchIsolatedElectron } from './helpers/launch'

const { app } = await launchIsolatedElectron({
  sessionBase, // optional; defaults to a fresh mkdtempSync
  userDataDir: udd // optional; defaults to a fresh mkdtempSync
})
```

What the helper guarantees:

- Unique `BPMUX_BACKEND_PORT` via `getFreePort` — no collision with
  Ryan's live backend on 4117 or with parallel test processes.
- Unique `BPMUX_SESSION_BASE` temp dir — test fixture data can't
  leak into `~/.bpmux/`.
- Unique Electron `--user-data-dir` — localStorage / cookies /
  indexedDB start empty, no state bleed between tests.
- Optional `seedMessages` — writes a `messages-current.jsonl` under
  the session base so the spawned bpmux serves seeded content via
  `/api/messages` with zero mocking.

See `tests/port-isolation.spec.ts` for the substrate smoke test that
verifies these guarantees.

## Why this matters

2026-04-22: `pkill -f "Electron.app"` in test prep killed Ryan's
live app on every run (feedback memory: "Never broad-pkill during a
live shared session"). The port-collision class is adjacent: a test
that binds 4117 either fails to start or adopts Ryan's live backend
and writes fixture messages into his real chat log. The helper
closes both paths by assigning a fresh ephemeral port per launch.

## Running specs

```bash
npx playwright test                           # all
npx playwright test desk-check-390            # one spec file
npx playwright test desk-check-390 -g "click" # one test by grep
```

Specs run serially (`workers: 1`) in `playwright.config.ts` because
Electron instances can't parallel on the same port — but the helper
assigns per-launch ports, so future parallel modes won't collide
either.
