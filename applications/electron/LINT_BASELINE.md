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
- Custom-rule authoring for JSX `{' '}` detection and div/span-where-semantic-fits — covered by jsx-a11y existing rules + can be augmented as canon-violations surface in audit cycles
