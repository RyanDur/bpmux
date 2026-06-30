# Board & Backlog — two entities over one story lifecycle

*Domain model derived with Ryan, 2026-06-30. Captures the settled
entity boundary so the board/backlog UI (a future feature) and the
backend read-model split (a dev refactor) build on the same ground.*

## The shape

There is **one** story lifecycle — the `StoryState` discriminated union
(`Icebox`, `Ready`, `InProgress(Stage)`, `Delivered`, `Accepted`,
`Retired`; `Stage` is `Dev | DeskCheck`). Two distinct entities read
that one lifecycle, each projecting the part it cares about.

### Backlog — the ledger

The backlog is the ledger of record for every story across its whole
life. It is where work is authored and tracked:

- **draft** — the *act* of writing a story (not a state); a drafted
  story lands in `icebox`.
- **icebox** — captured but not ready.
- **ready** — promoted: ready for development.
- **in progress** — being worked.
- **finished / retired** — `Accepted` (finished) or `Retired`.

The backlog shows the **coarse** rung. "In progress" is a single rung
here: it rolls up every active state — `InProgress(Dev)`,
`InProgress(DeskCheck)`, and `Delivered`/acceptance all read as just
"in progress," because they are all active-not-finished. Only
`Accepted` reads as finished; `Retired` as retired.

### Board — the operational projection

The board **takes the current iteration** and projects that slice of
the ledger into the **detailed** working lanes, so the team can *do*
the work and run the gated workflow:

- **Ready** — the start lane (scheduling a story into the current
  iteration is the act of entering Ready).
- **In progress** — a detailed state machine: `InProgress(Dev)` →
  `InProgress(DeskCheck)`, with the gated transitions between stages.
- **Acceptance** — the stop lane (`Delivered`): awaiting accept/reject.

A story is *on the board* iff it is a member of the current iteration.
The board does not own story records — it is a projection over the
ledger, scoped to the current iteration.

## One state, two readers

`InProgress(Stage)` is the coarse/fine distinction encoded in the type:

| `StoryState`              | Backlog rung   | Board lane               |
|---------------------------|----------------|--------------------------|
| `Icebox`                  | icebox         | (not on board)           |
| `Ready`                   | ready          | Ready                    |
| `InProgress(Dev)`         | in progress    | In Progress · Dev        |
| `InProgress(DeskCheck)`   | in progress    | In Progress · Desk-check |
| `Delivered`               | in progress    | Acceptance               |
| `Accepted`                | finished       | (leaves the board)       |
| `Retired`                 | retired        | (not on board)           |

The board reads `Stage` and the transitions; the backlog ignores the
stage and reads `InProgress(_)`. No new states, no wire-string changes
— the same lifecycle, projected two ways.

## What is refactor vs. what is feature

- **Dev refactor (backend):** derive two read models over the ledger —
  a *board* read (current iteration → detailed lanes) and a *backlog*
  read (coarse ledger). Today a single board endpoint returns every
  story in every state and the renderer builds all columns from it;
  the refactor separates those two reads. Behavior-preserving until the
  UI catches up.
- **New feature (PM/designer):** the board/backlog **UI** — a board
  view showing only the current iteration's lanes, and a separate
  backlog view for the ledger (authoring, icebox, finished work). This
  is a visible product change the designer shapes and pm files; it is
  not part of the refactor.
