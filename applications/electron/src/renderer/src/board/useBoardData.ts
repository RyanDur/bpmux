import { useMemo } from 'react'
import { buildMembershipIndex } from './iterationMembers'
import type { BoardIterationData } from './selectIterations'
import {
  GROUP_ORDER,
  groupStories,
  isActiveEpic,
  storyTypeFromLabels,
  type BoardResponse,
  type BoardStory,
  type ColumnState
} from './types'

export interface UseBoardDataResult {
  stories: BoardStory[]
  epics: BoardStory[]
  epicTitleByNumber: Map<number, string>
  iterationContainerStories: BoardStory[]
  flatBoardStories: BoardStory[]
  grouped: Map<ColumnState, BoardStory[]>
  navOrder: number[]
}

export const useBoardData = (
  data: BoardResponse | null,
  iterations: BoardIterationData[],
  effectiveReadyPosition?: (story: BoardStory) => number | null
): UseBoardDataResult => {
  const stories = useMemo(() => data?.stories ?? [], [data])
  const membershipIndex = useMemo(() => buildMembershipIndex(iterations), [iterations])
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
        if (storyTypeFromLabels(s.labels) === 'iteration') return false
        if (s.state === 'icebox') return true
        if (membershipIndex.has(s.number)) return true
        return false
      }),
    [nonEpicStories, membershipIndex]
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
  const grouped = useMemo(() => {
    const byColumn = groupStories(flatBoardStories)
    const ready = byColumn.get('ready')
    if (ready) {
      const positionOf = (s: BoardStory): number | null =>
        effectiveReadyPosition
          ? effectiveReadyPosition(s)
          : (s.iteration_membership?.position ?? null)
      ready.sort((a, b) => {
        const ap = positionOf(a)
        const bp = positionOf(b)
        if (ap !== null && bp !== null) return ap - bp
        if (ap !== null) return -1
        if (bp !== null) return 1
        return a.number - b.number
      })
    }
    return byColumn
  }, [flatBoardStories, effectiveReadyPosition])
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
