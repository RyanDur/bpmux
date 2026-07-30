import { type Maybe, maybe } from '@ryandur/sand'
import { useSelector } from '../../selecting'
import type {
  CiSnapshotWire,
  ReactionAggregateWire,
  RosterMemberWire,
  SyncStatusWire
} from '@transport/types'

export const useRoster = (): RosterMemberWire[] => useSelector((state) => state.mirror.roster)

export const useReactionsByChannel = (): Readonly<Record<string, ReactionAggregateWire[]>> =>
  useSelector((state) => state.mirror.reactions)

export const useCiSnapshot = (): Maybe<CiSnapshotWire> => {
  const snapshot = useSelector((state) => state.mirror.ci)
  return maybe(snapshot)
}

export const useSyncStatus = (): Maybe<SyncStatusWire> => {
  const status = useSelector((state) => state.mirror.sync)
  return maybe(status)
}
