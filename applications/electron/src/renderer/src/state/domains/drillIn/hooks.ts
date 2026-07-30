import { type Maybe, maybe } from '@ryandur/sand'
import { useSelector } from '../../selecting'
import type { CommentWire, TimelineEventWire } from '@transport/types'

export const useComments = (storyId: number): Maybe<CommentWire[]> => {
  const comments = useSelector((state) => state.commentsByStory[storyId])
  return maybe(comments)
}

export const useTimelineEvents = (storyId: number): Maybe<TimelineEventWire[]> => {
  const events = useSelector((state) => state.timelineByStory[storyId])
  return maybe(events)
}
