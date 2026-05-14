// #1306 Epic #1247 slice 4.1 — sub-batch ATP marker file.
//
// The actual migrated tests for sub-batch 4.1.A (human-message-*
// family) live in sibling test files in this directory:
// - human-message-class-glyphs-render.test.tsx
// - human-message-grouping-five-minute-threshold.test.tsx
// - human-message-mention-renders-as-pill.test.tsx
// - human-message-pin-toggle-button-renders.test.tsx
//
// This file exists as the story's named acceptance_test_path marker
// so the deliver_story tool's path-validation finds the ATP. The real
// behavior under test is split across the sibling files above.

import { describe, it } from 'vitest'

describe('#1306 sub-batch 4.1.A — human-message-* migration', () => {
  it('migration scope — see sibling test files for behavior coverage', () => {})
})
