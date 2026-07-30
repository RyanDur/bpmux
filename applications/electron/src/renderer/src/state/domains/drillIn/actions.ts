import type { CommentWire, TimelineEventWire } from '@transport/types'

export enum DrillInActionType {
  CommentsHydrateRequested = 'state/commentsHydrateRequested',
  CommentsHydrated = 'state/commentsHydrated',
  TimelineHydrateRequested = 'state/timelineHydrateRequested',
  TimelineHydrated = 'state/timelineHydrated'
}

export type DrillInAction =
  | { type: DrillInActionType.CommentsHydrateRequested; storyId: number }
  | { type: DrillInActionType.CommentsHydrated; storyId: number; comments: CommentWire[] }
  | { type: DrillInActionType.TimelineHydrateRequested; storyId: number }
  | { type: DrillInActionType.TimelineHydrated; storyId: number; events: TimelineEventWire[] }

export const commentsHydrateRequested = (storyId: number): DrillInAction => ({
  type: DrillInActionType.CommentsHydrateRequested,
  storyId
})

export const commentsHydrated = (storyId: number, comments: CommentWire[]): DrillInAction => ({
  type: DrillInActionType.CommentsHydrated,
  storyId,
  comments
})

export const timelineHydrateRequested = (storyId: number): DrillInAction => ({
  type: DrillInActionType.TimelineHydrateRequested,
  storyId
})

export const timelineHydrated = (storyId: number, events: TimelineEventWire[]): DrillInAction => ({
  type: DrillInActionType.TimelineHydrated,
  storyId,
  events
})
