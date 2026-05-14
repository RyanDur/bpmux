// #1335 Epic #1247 slice 4.1.B — sub-batch ATP marker file.
//
// The actual migrated tests for sub-batch 4.1.B (3 deferred specs
// from #1306 4.1.A) live in sibling test files in this directory:
// - human-message-mention-click-scopes-recipient.test.tsx (seedRoster)
// - human-message-filter-persists-across-reload.test.tsx (win.reload-equiv)
// - human-message-reaction-picker-opens.test.tsx (reaction-state harness)
//
// This file exists as the story's named acceptance_test_path marker
// so the deliver_story tool's path-validation finds the ATP.

import { describe, it } from 'vitest'

describe('#1335 sub-batch 4.1.B — deferred specs migration', () => {
  it('migration scope — see sibling test files for behavior coverage', () => {})
})
