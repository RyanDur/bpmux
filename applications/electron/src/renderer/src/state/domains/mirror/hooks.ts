import { useSelector } from '../../selecting'
import type {
  CiSnapshotWire,
  ReactionAggregateWire,
  RosterMemberWire,
  SyncStatusWire
} from '@transport/types'

export const useRoster = (): RosterMemberWire[] => useSelector((s) => s.mirror.roster)

export const useReactionsByChannel = (): ReadonlyMap<string, ReactionAggregateWire[]> =>
  useSelector((s) => s.mirror.reactions)

export const useCiSnapshot = (): CiSnapshotWire | null => useSelector((s) => s.mirror.ci)

export const useSyncStatus = (): SyncStatusWire | null => useSelector((s) => s.mirror.sync)
