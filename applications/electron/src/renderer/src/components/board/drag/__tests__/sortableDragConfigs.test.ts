import { describe, expect, test } from 'vitest'
import { DRAG_MIMETYPE_REGISTRY } from '../useStoryDrag'
import { ITERATION_MEMBER_DRAG, READY_STORY_DRAG } from '../sortableDragConfigs'

describe('drag-mime root-cause guard', () => {
  test('every registered drag mime is an application/x- type, because a real-OS drag strips custom text/* subtypes from dataTransfer.types during dragover and kills the felt drag', () => {
    for (const mime of Object.values(DRAG_MIMETYPE_REGISTRY)) {
      expect(mime.startsWith('application/x-')).toBe(true)
    }
  })

  test('the ready-queue and member-list drag configs both reference application/x- mimes, so the ready-queue cannot drift dead the way text/bpmux-story did', () => {
    expect(READY_STORY_DRAG.mimeType.startsWith('application/x-')).toBe(true)
    expect(ITERATION_MEMBER_DRAG.mimeType.startsWith('application/x-')).toBe(true)
  })
})
