# Frontend Canon-Cluster Lint Baseline

Captured 2026-05-20 23:43 UTC as Slice 1 of Epic #1477 (story #1478). Established lint substrate (stylelint + extended eslint config with no-any / prefer-const / arrow-functions / jsx-a11y) and integrated into pre-commit hook + CI workflow on a **staged-files-only basis** — new code cannot introduce violations, existing baseline is tolerated until subsequent slices retire it.

## ESLint baseline (190 errors / 69 warnings)

| Count | Rule | Owning slice |
|------:|:-----|:-------------|
| 153 | `func-style` | Spread across renderer + tests; addressed per-component slices 2-8 |
| 9 | `react-refresh/only-export-components` | Slice 3 (Chat) + Slice 4 (Board) likely |
| 9 + 5 | `testing-library/render` family | Slice 5/9 cleanup |
| 3 | `@typescript-eslint/no-explicit-any` | Slice 4 (Board) likely |
| 2 | `no-empty` | Per-component touch |
| 2 | `jsx-a11y/no-noninteractive-tabindex` | Slice 2 (Sidebar) or Slice 4 (Board) |
| 2 | `jsx-a11y/no-noninteractive-element-interactions` | Slice 4 (Board) |
| 2 | `@typescript-eslint/explicit-function-return-type` | Per-component touch |
| 1 | `jsx-a11y/no-autofocus` | Slice 8 (Form/input atoms) |
| 1 | `jsx-a11y/click-events-have-key-events` | Per-component touch |
| 1 | `@typescript-eslint/no-unused-vars` | Per-component touch |

## ESLint Caelum extension — no-restricted-syntax (52 errors across 23 files) — armed 2026-05-21 (#1493)

`no-restricted-syntax` rule bans JSX `<div>` + `<span>` opening-elements in `src/renderer/src/**/*.tsx` (production-only; test + fixture files exempt) per Ryan-canon `feedback_html5_semantic_no_divs_spans` + `feedback_first_rule_of_aria`. Staged-files-only gate same as Slice 1 baseline — new code blocked at pre-commit; baseline tolerated until per-component slices touch each file.

| File | Count | Owning slice |
|:-----|------:|:-------------|
| `board/Epic.tsx` | 7 | Caelum #1455-rework (Epic-display) |
| `chat/Reactions.tsx` | 5 | Caelum Slice 3 / 7 (Chat / Reactions+UnfurlCards) |
| `chat/UnfurlCards.tsx` | 4 | Caelum Slice 7 |
| `sidebar/Sidebar.tsx` | 3 | Caelum #1457-rework + Slice 2 |
| `chat/MessageItem.tsx` | 3 | Caelum Slice 3 |
| `board/Tabs.tsx` | 3 | Caelum #1453-rework (Tabs HTML5+radio-group) |
| `board/Iteration.tsx` | 3 | Caelum Slice 4 |
| `chat/FilterPills.tsx` | 2 | Caelum #1456-rework |
| `board/Timeline.tsx` | 2 | Caelum Slice 4 |
| `board/StoryRow.tsx` | 2 | Caelum Slice 4 |
| `board/StoryBadge.tsx` | 2 | Caelum Slice 4 / 6 |
| `board/MiniBoard.tsx` | 2 | Caelum Slice 4 |
| `board/MemberList.tsx` | 2 | Caelum Slice 4 |
| `board/BoardGroup.tsx` | 2 | Caelum Slice 4 |
| `board/BoardResizeHandle.tsx` | 1 | Caelum Slice 4 |
| `board/ReworkGlyph.tsx` | 1 | Caelum Slice 4 / 6 |
| `board/UnscheduledList.tsx` | 1 | Caelum Slice 4 |
| `chat/ClassGlyph.tsx` | 1 | Caelum #1456-rework |
| `chat/Input.tsx` | 1 | Caelum Slice 3 / 8 |
| `chat/MessageBody.tsx` | 1 | Caelum Slice 3 |
| `chat/MessageList.tsx` | 1 | Caelum Slice 3 |
| `chat/renderMentions.tsx` | 1 | Caelum Slice 3 |
| `metrics/Metrics.tsx` | 1 | Caelum Slice 4 / catchall |

Waivers: none. Genuine library-edge waivers (if discovered later) should be added at the rule-site in `eslint.config.mjs` with `file:line + rationale` per #1493 falsifier (e).

## Stylelint baseline (166 errors)

| Count | Rule | Owning slice |
|------:|:-----|:-------------|
| 47 | `color-no-hex` | Slice 7 (Reactions + UnfurlCards + Metrics + sister) likely; extract to root tokens |
| 29 | `rule-empty-line-before` | Formatting; auto-fixable; per-component touch |
| 29 | `no-descending-specificity` | CSS architecture; addressed in component slices |
| 24 | `comment-empty-line-before` | Formatting; auto-fixable; per-component touch |
| 6 | `color-function-notation` | Auto-fixable |
| 6 | `color-function-alias-notation` | Auto-fixable |
| 6 | `alpha-value-notation` | Auto-fixable |
| 4 | `selector-pseudo-element-colon-notation` | Auto-fixable |
| 2 | `no-duplicate-selectors` | Per-component touch |
| 2 | `declaration-property-value-keyword-no-deprecated` | Per-component touch |
| 2 | `declaration-property-unit-allowed-list` | Unit-canon violations; per-component touch |
| 2 | `declaration-block-no-redundant-longhand-properties` | Per-component touch |
| 2 | `custom-property-empty-line-before` | Formatting |
| 2 | `color-hex-length` | Auto-fixable |
| 1 | `property-no-deprecated` | Per-component touch |
| 1 | `media-feature-range-notation` | Auto-fixable |
| 1 | `declaration-block-no-shorthand-property-overrides` | Per-component touch |

## Gate behavior

- **Pre-commit hook** (`scripts/lint-staged.sh` invoked by `scripts/git-hooks/pre-commit`): runs `npx eslint` + `npx stylelint` over STAGED files only. Any new violation introduced by the staged change blocks commit.
- **CI workflow** (`.github/workflows/ci.yml` `vitest-rtl` job): runs lint over CHANGED files in the PR/push diff. Same baseline-tolerated semantic.
- **Full-tree lint** (`npm run lint` + `npm run lint:css`): reports the full baseline; not gated. Slice 9 of Epic #1477 (`#1486`) audits the full tree = 0 violations as terminal verification.

## Out of scope for Slice 1

- Fixing the 356 baseline violations enumerated above (deferred to per-component slices 2-8)
- Full-tree gate enforcement (deferred to Slice 9 `#1486`)
- Custom-rule authoring for JSX `{' '}` detection (deferred); div/span-banned now armed via #1493 above.
