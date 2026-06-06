import type { SortableDragConfig } from './SortableList'
import type { DragPayload, IterationDragPayload } from './iterationMembers'
import { DRAG_MIMETYPE_REGISTRY } from './useStoryDrag'

const decodeMemberPayload = (raw: string): DragPayload | null => {
  try {
    return JSON.parse(raw) as DragPayload
  } catch {
    return null
  }
}

export const ITERATION_MEMBER_DRAG: SortableDragConfig<DragPayload> = {
  mimeType: DRAG_MIMETYPE_REGISTRY.ITERATION_MEMBER,
  encode: JSON.stringify,
  decode: decodeMemberPayload,
  payloadKey: (payload) => payload.storyNumber
}

const decodeStoryNumber = (raw: string): number | null => {
  const n = Number.parseInt(raw, 10)
  return Number.isNaN(n) ? null : n
}

export const READY_STORY_DRAG: SortableDragConfig<number> = {
  mimeType: DRAG_MIMETYPE_REGISTRY.STORY_ROW,
  encode: String,
  decode: decodeStoryNumber,
  payloadKey: (storyNumber) => storyNumber
}

const decodeIterationPayload = (raw: string): IterationDragPayload | null => {
  try {
    return JSON.parse(raw) as IterationDragPayload
  } catch {
    return null
  }
}

export const ITERATION_ITSELF_DRAG: SortableDragConfig<IterationDragPayload> = {
  mimeType: DRAG_MIMETYPE_REGISTRY.ITERATION_ITSELF,
  encode: JSON.stringify,
  decode: decodeIterationPayload,
  payloadKey: (payload) => payload.iterationNumber
}
