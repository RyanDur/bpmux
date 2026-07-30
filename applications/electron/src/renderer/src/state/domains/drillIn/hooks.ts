import { useSelector } from '../../selecting'
import type { CommentWire, TimelineEventWire } from '@transport/types'

export const useComments = (storyId: number): CommentWire[] | null =>
  useSelector((s) => s.commentsByStory.get(storyId) ?? null)

export const useTimelineEvents = (storyId: number): TimelineEventWire[] | null =>
  useSelector((s) => s.timelineByStory.get(storyId) ?? null)
