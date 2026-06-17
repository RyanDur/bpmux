import { useMemo } from 'react'
import Epic from './Epic'
import Iteration from './Iteration'
import BoardGroup from './BoardGroup'
import Roadmap from './Roadmap'
import Tabs, { type TabDef } from './Tabs'
import { useTabStorage } from '../lib/useTabStorage'
import { RENDERER_IDENTITY } from '../transport/useWebSocket'
import { epicNumbersInIteration } from './currentIteration'
import { deriveNowView } from './deriveNowView'
import {
  GROUP_ORDER,
  isActiveEpic,
  type BoardStory,
  type ColumnState,
  type GroupSpec
} from './types'
import './BoardTabs.css'

export type BoardTabId = 'now' | 'plan' | 'done'

const BOARD_TAB_STORAGE_PREFIX = 'bpmux-board-active-tab'
const VALID_TAB_IDS: ReadonlyArray<BoardTabId> = ['now', 'plan', 'done'] as const

const DONE_STATES: ReadonlyArray<ColumnState> = ['accepted', 'retired']

const NOW_STATES: ReadonlyArray<ColumnState> = ['ready', 'in-progress', 'desk-check', 'delivered']

const groupsFor = (states: ReadonlyArray<ColumnState>): GroupSpec[] =>
  states
    .map((s) => GROUP_ORDER.find((g) => g.state === s))
    .filter((g): g is GroupSpec => g !== undefined)

const iceboxGroup = (): GroupSpec | undefined => GROUP_ORDER.find((g) => g.state === 'icebox')

interface BoardTabsProps {
  stories: BoardStory[]
  epics: BoardStory[]
  epicTitleByNumber: Map<number, string>
  iterationContainerStories: BoardStory[]
  grouped: Map<ColumnState, BoardStory[]>
  loading: boolean
  effectiveSelected: number | null
  collapsed: Record<ColumnState, boolean>
  onToggleCollapse: (state: ColumnState) => void
  onOpen: (n: number) => void
  onOpenTimeline: (n: number) => void
  onSelect: (n: number) => void
  onReorder: (state: ColumnState, moved: number, targetNumber: number) => void
  onReorderAnnounce: (text: string) => void
}

const BoardTabs = ({
  stories,
  epics,
  epicTitleByNumber,
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

  const nowView = useMemo(() => deriveNowView(stories), [stories])
  const currentIteration = nowView.kind === 'current' ? nowView.iteration : null
  const currentIterationEpicNumbers = useMemo(() => {
    if (currentIteration === null) return []
    return epicNumbersInIteration(stories, currentIteration)
  }, [stories, currentIteration])
  const currentIterationEpics = useMemo(() => {
    if (currentIteration === null) return []
    const inIter = new Set(currentIterationEpicNumbers)
    return stories.filter((s) => isActiveEpic(s) && inIter.has(s.number))
  }, [stories, currentIteration, currentIterationEpicNumbers])
  const currentIterationMemberNumbers = useMemo<Set<number>>(
    () => new Set(nowView.kind === 'current' ? nowView.members.map((m) => m.story.number) : []),
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
    storiesOverride?: BoardStory[]
  ): React.JSX.Element => (
    <BoardGroup
      key={group.state}
      group={group}
      stories={
        storiesOverride ??
        (group.state === 'icebox' ? iterationContainerStories : (grouped.get(group.state) ?? []))
      }
      epicTitleByNumber={epicTitleByNumber}
      loading={loading}
      selectedNumber={effectiveSelected}
      onOpen={onOpen}
      onOpenTimeline={onOpenTimeline}
      onSelect={onSelect}
      onReorder={onReorder}
      collapsed={collapsed[group.state]}
      onToggleCollapse={() => onToggleCollapse(group.state)}
    >
      {children}
    </BoardGroup>
  )

  const renderNow = (): React.JSX.Element => {
    if (currentIteration === null) {
      return (
        <section className="board-tabpanel-content" data-test="board-tabpanel-now-content">
          <Roadmap stories={stories} />
        </section>
      )
    }
    return (
      <section className="board-tabpanel-content" data-test="board-tabpanel-now-content">
        <Epic epics={currentIterationEpics} stories={stories} onOpen={onOpen} />
        {groupsFor(NOW_STATES).map((group) =>
          renderGroup(
            group,
            undefined,
            (grouped.get(group.state) ?? []).filter((s) =>
              currentIterationMemberNumbers.has(s.number)
            )
          )
        )}
      </section>
    )
  }

  const renderPlan = (): React.JSX.Element => {
    const icebox = iceboxGroup()
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
      {groupsFor(DONE_STATES).map((group) => renderGroup(group))}
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
