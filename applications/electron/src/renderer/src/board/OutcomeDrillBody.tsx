import type { DrillIteration, OutcomeDrillIn, ConveneCheckState } from './deriveOutcomeDrillIn'

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
  outcomeNumber,
  drill
}: {
  outcomeNumber: number
  drill: OutcomeDrillIn
}): React.JSX.Element => (
  <section className="roadmap-drill" data-test={`roadmap-drill-${outcomeNumber}`}>
    {drill.convene !== null && (
      <p className="roadmap-convene" data-test={`roadmap-convene-${outcomeNumber}`}>
        <b>CONVENE?</b> next iteration <b>{drill.convene.nextIterationName}</b>
        {conveneTail(drill)}
      </p>
    )}
    <p className="roadmap-drill-title">
      The build-up — iterations producing this outcome, in order
    </p>
    <ol className="roadmap-drill-iterations">
      {drill.iterations.map((iteration) => (
        <li
          key={iteration.number}
          className="roadmap-drill-iteration"
          data-test={`roadmap-drill-iteration-${iteration.number}`}
          data-state={iteration.state}
        >
          <p className="roadmap-drill-iteration-head">
            <b className="roadmap-drill-iteration-name" data-test="roadmap-drill-iteration-name">
              {iteration.name}
            </b>
            <small className="roadmap-drill-iteration-state">{iterationStateText(iteration)}</small>
          </p>
          {iteration.checks.length > 0 && (
            <ul
              className="roadmap-drill-checks"
              data-test={`roadmap-drill-checks-${iteration.number}`}
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
      ))}
    </ol>
  </section>
)

export default OutcomeDrillBody
