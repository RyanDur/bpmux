import { useSyncExternalStore } from 'react'
import { getStateSnapshot, subscribeState } from './store'
import type {
  ChatMessage,
  CiSnapshotWire,
  CommentWire,
  ConnectionStatus,
  ReactionAggregateWire,
  RosterMemberWire,
  SyncStatusWire,
  TimelineEventWire
} from '../transport/types'
import type { BoardColumn, Story } from '../board/types'
const useSelector = <T>(selector: (snapshot: ReturnType<typeof getStateSnapshot>) => T): T => {
  const snapshot = (): T => selector(getStateSnapshot())
  return useSyncExternalStore(subscribeState, snapshot, snapshot)
}

export const useConnectionStatus = (): ConnectionStatus => useSelector((s) => s.status)

export const useLiveMessages = (): ChatMessage[] => useSelector((s) => s.messages)

export const useRoster = (): RosterMemberWire[] => useSelector((s) => s.mirror.roster)

export const useReactionsByChannel = (): ReadonlyMap<string, ReactionAggregateWire[]> =>
  useSelector((s) => s.mirror.reactions)

export const useCiSnapshot = (): CiSnapshotWire | null => useSelector((s) => s.mirror.ci)

export const useSyncStatus = (): SyncStatusWire | null => useSelector((s) => s.mirror.sync)

export const useStories = (): Story[] => useSelector((s) => s.stories)

export const useBoardColumns = (): BoardColumn[] => useSelector((s) => s.columns)

export const useBoardInvalidateTick = (): number => useSelector((s) => s.boardInvalidateTick)

export const useComments = (storyId: number): CommentWire[] | null =>
  useSelector((s) => s.commentsByStory.get(storyId) ?? null)

export const useTimelineEvents = (storyId: number): TimelineEventWire[] | null =>
  useSelector((s) => s.timelineByStory.get(storyId) ?? null)
