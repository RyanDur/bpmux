import { useMemo } from 'react'
import { buildMembershipIndex } from './iterationMembers'
import type { BoardIterationData } from './iteration/selectIterations'
import {
  groupOrderOrFallback,
  groupStoriesIn,
  isActiveEpic,
  middleColumnIds,
  storyTypeFromLabels,
  type BoardColumn,
  type Epic,
  type GroupSpec,
  type Story,
  type ColumnState
} from './types'

export type UseBoardDataResult = {
  stories: Story[]
  epics: Epic[]
  epicTitleById: Map<number, string>
  iterationContainerStories: Story[]
  flatBoardStories: Story[]
  grouped: Map<ColumnState, Story[]>
  groupOrder: GroupSpec[]
  navOrder: number[]
}

export const useBoardData = (
  data: Story[] | null,
  iterations: BoardIterationData[],
  readyOrder?: number[],
  wireEpics: Epic[] = [],
  columns: BoardColumn[] = []
): UseBoardDataResult => {
  const stories = useMemo(() => data ?? [], [data])
  const groupOrder = useMemo(() => groupOrderOrFallback(columns), [columns])
  const middleIds = useMemo(() => middleColumnIds(groupOrder), [groupOrder])
  const membershipIndex = useMemo(() => buildMembershipIndex(iterations), [iterations])
  const epics = useMemo(() => wireEpics.filter(isActiveEpic), [wireEpics])
  const epicTitleById = useMemo(() => {
    const map = new Map<number, string>()
    for (const e of wireEpics) map.set(e.id, e.title)
    return map
  }, [wireEpics])
  const iterationContainerStories = useMemo(
    () =>
      stories.filter((s) => {
        if (storyTypeFromLabels(s.labels) === 'iteration') return false
        if (s.state === 'icebox') return true
        if (membershipIndex.has(s.id)) return true
        return false
      }),
    [stories, membershipIndex]
  )
  const flatBoardStories = useMemo(
    () =>
      stories.filter((s) => {
        if (storyTypeFromLabels(s.labels) === 'iteration') return false
        if (s.state === 'icebox') return false
        return true
      }),
    [stories]
  )
  const grouped = useMemo(() => {
    const byColumn = groupStoriesIn(middleIds, flatBoardStories)
    const ready = byColumn.get('ready')
    if (ready) {
      const order = readyOrder ?? []
      const rankOf = (s: Story): number => {
        const i = order.indexOf(s.id)
        return i === -1 ? Number.MAX_SAFE_INTEGER : i
      }
      ready.sort((a, b) => {
        const ar = rankOf(a)
        const br = rankOf(b)
        if (ar !== br) return ar - br
        return a.id - b.id
      })
    }
    return byColumn
  }, [middleIds, flatBoardStories, readyOrder])
  const navOrder = useMemo(() => {
    const out: number[] = []
    for (const group of groupOrder) {
      const list = grouped.get(group.state) ?? []
      for (const s of list) out.push(s.id)
    }
    return out
  }, [groupOrder, grouped])
  return {
    stories,
    epics,
    epicTitleById,
    iterationContainerStories,
    flatBoardStories,
    grouped,
    groupOrder,
    navOrder
  }
}
