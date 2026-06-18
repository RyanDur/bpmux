import { useSyncExternalStore } from 'react'
import { getSessionSnapshot, subscribeSession } from './store'
import type {
  ChatMessage,
  CiSnapshotWire,
  ConnectionStatus,
  ReactionAggregateWire,
  RosterMemberWire,
  SyncStatusWire
} from '../types'
import type { BoardResponse } from '../../board/types'

const useSelector = <T>(selector: (snapshot: ReturnType<typeof getSessionSnapshot>) => T): T => {
  const snapshot = (): T => selector(getSessionSnapshot())
  return useSyncExternalStore(subscribeSession, snapshot, snapshot)
}

export const useConnectionStatus = (): ConnectionStatus => useSelector((s) => s.status)

export const useSessionId = (): string | null => useSelector((s) => s.sessionId)

export const useLiveMessages = (): ChatMessage[] => useSelector((s) => s.messages)

export const useRoster = (): RosterMemberWire[] => useSelector((s) => s.mirror.roster)

export const useReactionsByChannel = (): ReadonlyMap<string, ReactionAggregateWire[]> =>
  useSelector((s) => s.mirror.reactions)

export const useCiSnapshot = (): CiSnapshotWire | null => useSelector((s) => s.mirror.ci)

export const useSyncStatus = (): SyncStatusWire | null => useSelector((s) => s.mirror.sync)

export const useBoardSnapshot = (): BoardResponse | null => useSelector((s) => s.boardSnapshot)

export const useBoardInvalidateTick = (): number => useSelector((s) => s.boardInvalidateTick)
