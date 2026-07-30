import { maybe } from '@ryandur/sand'
import { useSelector } from '../../selecting'
import type { CommentWire, TimelineEventWire } from '@transport/types'

export const useComments = (storyId: number): CommentWire[] | null =>
  useSelector((s) => maybe(s.commentsByStory.get(storyId)).orNull())

export const useTimelineEvents = (storyId: number): TimelineEventWire[] | null =>
  useSelector((s) => maybe(s.timelineByStory.get(storyId)).orNull())
