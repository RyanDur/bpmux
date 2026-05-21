/**
 * #1366 single-source for board-axis data-test selector strings.
 *
 * Pre-Andromeda, `data-test="story-grip"` / `data-test="iteration-grip"`
 * / `data-test={`board-iteration-${number}`}` literals were duplicated
 * across component-source AND test-files. Drift between component +
 * test would silently break drag/grip-related RTL specs. Sister to
 * `labelSchema.ts` (#1365) + Ryan-canon "magic numbers and strings."
 *
 * Use these constants from BOTH component-render-sites AND RTL test
 * queries so the value lives in ONE place.
 */

export const STORY_GRIP_TEST_ID = 'story-grip'
export const ITERATION_GRIP_TEST_ID = 'iteration-grip'

export const boardIterationTestId = (iterationNumber: number): string =>
  `board-iteration-${iterationNumber}`
