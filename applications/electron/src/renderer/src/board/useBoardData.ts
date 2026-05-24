import { useMemo } from 'react'
import { isIterationPositionLabel } from './labelSchema'
import {
  GROUP_ORDER,
  groupStories,
  isActiveEpic,
  storyTypeFromLabels,
  type BoardResponse,
  type BoardStory,
  type ColumnState
} from './types'

const isIterationMember = (story: BoardStory): boolean => {
  if (story.iteration_membership != null) return true
  return story.labels.some((l) => isIterationPositionLabel(l))
}

export interface UseBoardDataResult {
  stories: BoardStory[]
  epics: BoardStory[]
  epicTitleByNumber: Map<number, string>
  iterationContainerStories: BoardStory[]
  flatBoardStories: BoardStory[]
  grouped: Map<ColumnState, BoardStory[]>
  navOrder: number[]
}

export const useBoardData = (data: BoardResponse | null): UseBoardDataResult => {
  const stories = useMemo(() => data?.stories ?? [], [data])
  const epics = useMemo(() => stories.filter(isActiveEpic), [stories])
  const epicTitleByNumber = useMemo(() => {
    const map = new Map<number, string>()
    for (const s of stories) {
      if (storyTypeFromLabels(s.labels) === 'epic') map.set(s.number, s.title)
    }
    return map
  }, [stories])
  const nonEpicStories = useMemo(
    () => stories.filter((s) => storyTypeFromLabels(s.labels) !== 'epic'),
    [stories]
  )
  const iterationContainerStories = useMemo(
    () =>
      nonEpicStories.filter((s) => {
        if (storyTypeFromLabels(s.labels) === 'iteration') return true
        if (s.state === 'icebox') return true
        if (isIterationMember(s)) return true
        return false
      }),
    [nonEpicStories]
  )
  const flatBoardStories = useMemo(
    () =>
      nonEpicStories.filter((s) => {
        if (storyTypeFromLabels(s.labels) === 'iteration') return false
        if (s.state === 'icebox') return false
        return true
      }),
    [nonEpicStories]
  )
  const grouped = useMemo(() => groupStories(flatBoardStories), [flatBoardStories])
  const navOrder = useMemo(() => {
    const out: number[] = []
    for (const group of GROUP_ORDER) {
      const list = grouped.get(group.state) ?? []
      for (const s of list) out.push(s.number)
    }
    return out
  }, [grouped])
  return {
    stories,
    epics,
    epicTitleByNumber,
    iterationContainerStories,
    flatBoardStories,
    grouped,
    navOrder
  }
}
