import { useSelector } from '../../selecting'
import type { BoardColumn, Story } from '@components/board/types'

export const useStories = (): Story[] => useSelector((s) => s.stories)

export const useBoardColumns = (): BoardColumn[] => useSelector((s) => s.columns)

export const useBoardInvalidateTick = (): number => useSelector((s) => s.boardInvalidateTick)
