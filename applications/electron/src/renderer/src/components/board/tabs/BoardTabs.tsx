import { useMemo } from 'react'
import Epic from '../roadmap/Epic'
import Iteration from '../iteration/Iteration'
import BoardGroup from '../BoardGroup'
import Roadmap from '../roadmap/Roadmap'
import Tabs, { type TabDef } from './Tabs'
import { useTabStorage } from './useTabStorage'
import { RENDERER_IDENTITY } from '@state'
import { epicIdsInIteration } from '../iteration/currentIteration'
import { deriveNowView } from '../roadmap/deriveNowView'
import type { BoardIterationData } from '../iteration/selectIterations'
import {
  ACCEPTED_COLUMN,
  ICEBOX_COLUMN,
  RETIRED_COLUMN,
  type Epic as EpicData,
  type Story,
  type ColumnState,
  type GroupSpec
} from '../types'
import './BoardTabs.css'

type BoardTabId = 'now' | 'plan' | 'done'

const BOARD_TAB_STORAGE_PREFIX = 'bpmux-board-active-tab'
const VALID_TAB_IDS: ReadonlyArray<BoardTabId> = ['now', 'plan', 'done'] as const

const DONE_COLUMNS = new Set<ColumnState>([ACCEPTED_COLUMN, RETIRED_COLUMN])

const nowGroups = (groupOrder: GroupSpec[]): GroupSpec[] =>
  groupOrder.filter((g) => g.state !== ICEBOX_COLUMN && !DONE_COLUMNS.has(g.state))

const doneGroups = (groupOrder: GroupSpec[]): GroupSpec[] =>
  groupOrder.filter((g) => DONE_COLUMNS.has(g.state))

const iceboxGroup = (groupOrder: GroupSpec[]): GroupSpec | undefined =>
  groupOrder.find((g) => g.state === ICEBOX_COLUMN)

type BoardTabsProps = {
  stories: Story[]
  iterations: BoardIterationData[]
  groupOrder: GroupSpec[]
  epics: EpicData[]
  epicTitleById: Map<number, string>
  iterationContainerStories: Story[]
  grouped: Map<ColumnState, Story[]>
  loading: boolean
  effectiveSelected: number | null
  collapsed: Record<ColumnState, boolean>
  onToggleCollapse: (state: ColumnState) => void
  onOpen: (n: number) => void
  onOpenTimeline: (n: number) => void
  onSelect: (n: number) => void
  onReorder: (state: ColumnState, moved: number, targetId: number) => void
  onReorderAnnounce: (text: string) => void
}

const BoardTabs = ({
  stories,
  iterations,
  groupOrder,
  epics,
  epicTitleById,
  iterationContainerStories,
  grouped,
  loading,
  effectiveSelected,
  collapsed,
  onToggleCollapse,
  onOpen,
  onOpenTimeline,
  onSelect,
  onReorder,
  onReorderAnnounce
}: BoardTabsProps): React.JSX.Element => {
  const { activeId, setActiveId } = useTabStorage<BoardTabId>(
    BOARD_TAB_STORAGE_PREFIX,
    RENDERER_IDENTITY,
    VALID_TAB_IDS,
    'now'
  )

  const nowView = useMemo(() => deriveNowView(iterations, stories), [iterations, stories])
  const currentIteration = nowView.kind === 'current' ? nowView.iteration : null
  const currentIterationEpicIds = useMemo(() => {
    if (currentIteration === null) return []
    return epicIdsInIteration(stories, currentIteration)
  }, [stories, currentIteration])
  const currentIterationEpics = useMemo(() => {
    if (currentIteration === null) return []
    const inIter = new Set(currentIterationEpicIds)
    return epics.filter((e) => inIter.has(e.id))
  }, [epics, currentIteration, currentIterationEpicIds])
  const currentIterationMemberIds = useMemo<Set<number>>(
    () => new Set(nowView.kind === 'current' ? nowView.members.map((m) => m.story.id) : []),
    [nowView]
  )

  const tabDefs: ReadonlyArray<TabDef<BoardTabId>> = useMemo(
    () => [
      { id: 'now', label: 'Now', tabTestId: 'board-tab-now', panelTestId: 'board-tabpanel-now' },
      {
        id: 'plan',
        label: 'Plan',
        tabTestId: 'board-tab-plan',
        panelTestId: 'board-tabpanel-plan'
      },
      { id: 'done', label: 'Done', tabTestId: 'board-tab-done', panelTestId: 'board-tabpanel-done' }
    ],
    []
  )

  const renderGroup = (
    group: GroupSpec,
    children?: React.ReactNode,
    storiesOverride?: Story[]
  ): React.JSX.Element => (
    <BoardGroup
      key={group.state}
      group={group}
      stories={
        storiesOverride ??
        (group.state === 'icebox' ? iterationContainerStories : (grouped.get(group.state) ?? []))
      }
      epicTitleById={epicTitleById}
      loading={loading}
      selectedId={effectiveSelected}
      onOpen={onOpen}
      onOpenTimeline={onOpenTimeline}
      onSelect={onSelect}
      onReorder={onReorder}
      collapsed={collapsed[group.state] ?? group.collapsed}
      onToggleCollapse={() => onToggleCollapse(group.state)}
    >
      {children}
    </BoardGroup>
  )

  const renderNow = (): React.JSX.Element => {
    if (currentIteration === null) {
      return (
        <section className="board-tabpanel-content" data-test="board-tabpanel-now-content">
          <Roadmap />
        </section>
      )
    }
    return (
      <section className="board-tabpanel-content" data-test="board-tabpanel-now-content">
        <Epic epics={currentIterationEpics} stories={stories} onOpen={onOpen} />
        {nowGroups(groupOrder).map((group) =>
          renderGroup(
            group,
            undefined,
            (grouped.get(group.state) ?? []).filter((s) => currentIterationMemberIds.has(s.id))
          )
        )}
      </section>
    )
  }

  const renderPlan = (): React.JSX.Element => {
    const icebox = iceboxGroup(groupOrder)
    return (
      <section className="board-tabpanel-content" data-test="board-tabpanel-plan-content">
        <Epic epics={epics} stories={stories} onOpen={onOpen} />
        {icebox !== undefined &&
          renderGroup(
            icebox,
            <Iteration
              stories={iterationContainerStories}
              onOpen={onOpen}
              onReorderAnnounce={onReorderAnnounce}
            />
          )}
      </section>
    )
  }

  const renderDone = (): React.JSX.Element => (
    <section className="board-tabpanel-content" data-test="board-tabpanel-done-content">
      {doneGroups(groupOrder).map((group) => renderGroup(group))}
    </section>
  )

  const renderPanel = (id: BoardTabId): React.ReactNode => {
    switch (id) {
      case 'now':
        return renderNow()
      case 'plan':
        return renderPlan()
      case 'done':
        return renderDone()
    }
  }

  return (
    <Tabs<BoardTabId>
      ariaLabel="Board sections"
      tabs={tabDefs}
      activeId={activeId}
      onActiveChange={setActiveId}
      renderPanel={renderPanel}
      className="board-tabs"
      panelClassName="board-tabs-panel"
    />
  )
}

export default BoardTabs
