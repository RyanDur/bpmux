import { useMemo } from 'react'
import { buildMembershipIndex } from './iterationMembers'
import type { BoardIterationData } from './selectIterations'
import {
  GROUP_ORDER,
  groupStories,
  isActiveEpic,
  storyTypeFromLabels,
  type Story,
  type ColumnState
} from './types'

export interface UseBoardDataResult {
  stories: Story[]
  epics: Story[]
  epicTitleById: Map<number, string>
  iterationContainerStories: Story[]
  flatBoardStories: Story[]
  grouped: Map<ColumnState, Story[]>
  navOrder: number[]
}

export const useBoardData = (
  data: Story[] | null,
  iterations: BoardIterationData[],
  effectiveReadyPosition?: (story: Story) => number | null
): UseBoardDataResult => {
  const stories = useMemo(() => data ?? [], [data])
  const membershipIndex = useMemo(() => buildMembershipIndex(iterations), [iterations])
  const epics = useMemo(() => stories.filter(isActiveEpic), [stories])
  const epicTitleById = useMemo(() => {
    const map = new Map<number, string>()
    for (const s of stories) {
      if (storyTypeFromLabels(s.labels) === 'epic') map.set(s.id, s.title)
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
        if (membershipIndex.has(s.id)) return true
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
      const positionOf = (s: Story): number | null =>
        effectiveReadyPosition
          ? effectiveReadyPosition(s)
          : (s.iteration_membership?.position ?? null)
      ready.sort((a, b) => {
        const ap = positionOf(a)
        const bp = positionOf(b)
        if (ap !== null && bp !== null) return ap - bp
        if (ap !== null) return -1
        if (bp !== null) return 1
        return a.id - b.id
      })
    }
    return byColumn
  }, [flatBoardStories, effectiveReadyPosition])
  const navOrder = useMemo(() => {
    const out: number[] = []
    for (const group of GROUP_ORDER) {
      const list = grouped.get(group.state) ?? []
      for (const s of list) out.push(s.id)
    }
    return out
  }, [grouped])
  return {
    stories,
    epics,
    epicTitleById,
    iterationContainerStories,
    flatBoardStories,
    grouped,
    navOrder
  }
}
