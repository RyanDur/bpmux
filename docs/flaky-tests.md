# Flaky tests log

Tracked flaky tests (pass in isolation, fail nondeterministically under the
full parallel suite). Maintained by dev as flakes surface; feeds the de-flake
story (#1529 + extensions). Each entry: test, suite, vector, evidence,
occurrences.

Two suites surface flakes: the `integration_main` cargo test binary (~291
tests, parallel in one process, shared process-global / on-disk state) and the
vitest `npm test:unit` renderer suite. The cargo signature: **passes in
isolation, fails under full-binary parallelism, a DIFFERENT test fails each
run** (nondeterministic-which-test = contention, not a deterministic
regression). The vitest signature so far is **subprocess-spawning tests timing
out under load** (eslint/binary cascade), not assertion failures.

## Open — integration_main (cargo nextest)

### diag_routing::log_diag_under_cargo_test_writes_to_tmpdir_not_home
- **Suite:** integration_main
- **Vector:** shared on-disk file — `TEST_DIAG_FILENAME` is a const
  (`bpmux-test-diag.log`), so every parallel proc writes/reads one
  `${TMPDIR}/bpmux-test-diag.log`.
- **Evidence:** failed in CI on slice-20 (edee3f4); passed in isolation;
  full-binary local run failed a *different* test (accept_story_e2e).
- **Occurrences:** 2026-05-23 (CI slice-20, ×1 then green on rerun).
- **Fix candidate:** per-test unique diag tmpfile (not a shared const name).

### accept_story_e2e::accept_story_view_board_reflects_accepted_via_mocked_seed
- **Suite:** integration_main
- **Vector:** workflow-config / `allowed_roles:[ryan]` test-isolation
  (cross-test contamination). Already filed: **#1529** (icebox,
  `[Chore] De-flake accept_story_e2e — workflow-config test-isolation`).
- **Evidence:** failed in a local full-binary run 2026-05-23; passed in
  isolation.
- **Occurrences:** 2026-05-23 (local full-binary, ×1).
- **Fix candidate:** #1529 — workflow-config isolation per test.

### agent_side_assertions::accept_story_allows_pm_and_broadcasts_role_attribution
- **Suite:** integration_main
- **Vector:** SAME root-cause as #1529 (NOT a new vector) — workflow
  allowed-roles globally-mutable shared-state. PM-accept rejected with
  `allowed_roles:[ryan]` (the default/contaminated config) though the seeded
  story should allow PM. This is a 2nd *symptom* of the #1529 vector
  (its original symptom = accept_story_e2e); #1529's fix retires the
  globally-mutable allowed-roles state → covers BOTH tests. (Grounded in
  #1529's body, read 2026-05-23 — its falsifier names the shared-state.)
- **Evidence:** failed in CI on slice-47 (2f9fdad1); passed in isolation
  (1 passed); slice-47 was comment-deletion-only in an unrelated file
  (main/index.ts) + passed pre-commit + pre-push Playwright.
- **Occurrences:** 2026-05-23 (CI slice-47, ×1; rerun fired).
- **Fix candidate:** #1529's workflow-config isolation — its protect-test
  should pin BOTH accept-tests green under parallel.

## Open — vitest (npm test:unit)

### eslint-testing-library.test.ts > prefer-user-event fires on its anti-pattern in fixture
- **Suite:** vitest (npm test:unit), file
  `src/renderer/src/__tests__/eslint-testing-library.test.ts` (#1423 substrate)
- **Vector:** the test spawns eslint on a fixture (subprocess); the whole file
  took 8146ms while the per-test timeout is 5000ms. Under pre-commit load
  (parallel cargo recompile + nextest + vitest) the eslint subprocess starves
  → single test times out. Load-induced, NOT an assertion failure.
- **Evidence:** failed local pre-commit on slice-55 (CSS-only change, cannot
  affect eslint-rule tests); passed clean on immediate re-run (489 passed).
  Same run's nextest self-reported "1835 passed (1 flaky)".
- **Occurrences:** 2026-05-23 (local pre-commit slice-55, ×1, green on rerun).
- **Fix candidate:** raise per-test timeout for subprocess-spawning eslint
  tests (testTimeout override on the file/describe), or serialize them off the
  hot pre-commit path. Sister to the cargo-recompile-cascade vitest flakes.

## De-flake scope (for the N+1 story)

`integration_main` has ≥3 distinct parallelism / shared-state flake-vectors:
1. **Ports** — many e2e tests spawn bpmux + httpmock MockServer; port/server
   contention under load.
2. **Shared on-disk file** — the const `bpmux-test-diag.log` (diag_routing).
3. **Workflow-config / acceptor-axis shared-state** — `allowed_roles`
   contamination (accept_story_e2e #1529, agent_side_assertions).

Direction: per-test isolation (unique ports / tmpfiles / workflow-config) or a
shared-state reset between cases. A suite-wide hardening generalizes #1529.

Plus a 4th vector in a DIFFERENT suite — **vitest subprocess-timeout**: tests
that spawn eslint/a binary (eslint-testing-library.test.ts) starve under
pre-commit load and trip the 5s per-test timeout. Direction: per-file
testTimeout override or move subprocess tests off the hot pre-commit path.
