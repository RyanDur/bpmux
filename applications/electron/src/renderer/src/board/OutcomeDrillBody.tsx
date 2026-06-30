import type { DrillIteration, OutcomeDrillIn, ConveneCheckState } from './deriveOutcomeDrillIn'
import { SortableList } from './SortableList'
import { DragGrip } from './DragGrip'
import { OUTCOME_ITERATION_DRAG } from './sortableDragConfigs'
import { useDispatch } from '../transport/useDispatch'
import { outcomeIterationReorderRequested } from '../transport/session/actions'

const CHECK_STATE_WORD: Record<ConveneCheckState, string> = {
  met: 'met',
  gap: 'gap',
  pending: 'pending'
}

const gapText = (count: number): string => `${count} ${count === 1 ? 'gap' : 'gaps'}`

const iterationStateText = (iteration: DrillIteration): string => {
  switch (iteration.state) {
    case 'ready':
      return 'ready'
    case 'next':
      return `next · ${gapText(iteration.gapCount)}`
    case 'gap':
      return gapText(iteration.gapCount)
    case 'not-started':
      return 'not started'
  }
}

const conveneTail = (drill: OutcomeDrillIn): string => {
  if (drill.convene === null) return ''
  const active =
    drill.convene.activeGapLabel === null ? '' : ` (${drill.convene.activeGapLabel} open)`
  const then =
    drill.convene.thenIterationName === null ? '' : `; then ${drill.convene.thenIterationName}`
  return ` — ${gapText(drill.convene.nextGapCount)}${active}${then}`
}

const OutcomeDrillBody = ({
  outcomeId,
  drill
}: {
  outcomeId: number
  drill: OutcomeDrillIn
}): React.JSX.Element => {
  const dispatch = useDispatch()
  const reorder = (iterationId: number, targetKey: number): void => {
    const position = drill.iterations.findIndex((it) => it.id === targetKey) + 1
    if (position === 0) return
    dispatch(outcomeIterationReorderRequested(outcomeId, iterationId, position))
  }
  return (
    <section className="roadmap-drill" data-test={`roadmap-drill-${outcomeId}`}>
      {drill.convene !== null && (
        <p className="roadmap-convene" data-test={`roadmap-convene-${outcomeId}`}>
          <b>CONVENE?</b> next iteration <b>{drill.convene.nextIterationName}</b>
          {conveneTail(drill)}
        </p>
      )}
      <p className="roadmap-drill-title">
        The build-up — iterations producing this outcome, in order
      </p>
      <SortableList
        items={drill.iterations}
        keyOf={(iteration) => iteration.id}
        rowKey={(iteration) => iteration.id}
        config={OUTCOME_ITERATION_DRAG}
        buildPayload={(iteration) => iteration.id}
        onReorder={reorder}
        ulClassName="roadmap-drill-iterations"
        renderRow={(iteration, { rowDragProps, gripDragProps }) => (
          <li
            className="roadmap-drill-iteration"
            data-test={`roadmap-drill-iteration-${iteration.id}`}
            data-state={iteration.state}
            {...rowDragProps}
          >
            <p className="roadmap-drill-iteration-head">
              <DragGrip storyNumber={iteration.id} gripDragProps={gripDragProps} />
              <b className="roadmap-drill-iteration-name" data-test="roadmap-drill-iteration-name">
                {iteration.name}
              </b>
              <small className="roadmap-drill-iteration-state">
                {iterationStateText(iteration)}
              </small>
            </p>
            {iteration.checks.length > 0 && (
              <ul
                className="roadmap-drill-checks"
                data-test={`roadmap-drill-checks-${iteration.id}`}
              >
                {iteration.checks.map((check) => (
                  <li
                    key={check.key}
                    className="roadmap-drill-check"
                    data-state={check.state}
                    aria-label={`${check.label}: ${CHECK_STATE_WORD[check.state]}`}
                  >
                    {check.label}
                  </li>
                ))}
              </ul>
            )}
          </li>
        )}
      />
    </section>
  )
}

export default OutcomeDrillBody
