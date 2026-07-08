import { Fragment, useState } from 'react'
import { useRoadmap } from './useRoadmap'
import { deriveOutcomeDrillIn } from './deriveOutcomeDrillIn'
import OutcomeDrillBody from './OutcomeDrillBody'
import { SortableList } from './SortableList'
import { DragGrip } from './DragGrip'
import type { GripDragProps } from './useDragReorder'
import { OUTCOME_REPRIORITIZE_DRAG } from './sortableDragConfigs'
import type { OutcomeWhy, RoadmapOutcome } from './selectRoadmap'
import type { Horizon } from './types'
import { useDispatch } from '../transport/useDispatch'
import {
  outcomeHorizonChangeRequested,
  outcomeReprioritizeRequested
} from '../transport/state/actions'
import './Roadmap.css'

const META_FIELDS: ReadonlyArray<{ key: keyof OutcomeWhy; label: string }> = [
  { key: 'metric', label: 'Metric' },
  { key: 'confidence', label: 'Confidence' },
  { key: 'value', label: 'Value' }
]

const HORIZONS: ReadonlyArray<{ horizon: Horizon; label: string }> = [
  { horizon: 'now', label: 'Now' },
  { horizon: 'next', label: 'Next' },
  { horizon: 'later', label: 'Later' }
]

const readinessText = (outcome: RoadmapOutcome): string | null => {
  switch (outcome.readinessState) {
    case 'ready':
      return 'ready'
    case 'gap':
      return `${outcome.gapCount} ${outcome.gapCount === 1 ? 'gap' : 'gaps'}`
    case 'none':
      return null
  }
}

const fieldText = (value: string | undefined): string =>
  value === '' || value === undefined ? '—' : value

const RoadmapOutcomeCard = ({
  outcome,
  horizon,
  focused,
  onToggle,
  gripDragProps
}: {
  outcome: RoadmapOutcome
  horizon: Horizon
  focused: boolean
  onToggle: (outcomeId: number) => void
  gripDragProps: GripDragProps
}): React.JSX.Element => {
  const dispatch = useDispatch()
  const readiness = readinessText(outcome)
  const presentMeta = META_FIELDS.filter(({ key }) => key in outcome.why)
  return (
    <article
      className="roadmap-outcome"
      data-test={`roadmap-outcome-${outcome.id}`}
      data-tone={outcome.tone}
      data-focused={focused || undefined}
    >
      <header className="roadmap-outcome-head">
        <DragGrip storyId={outcome.id} gripDragProps={gripDragProps} />
        <h4 className="roadmap-outcome-name">
          <button
            type="button"
            className="roadmap-outcome-toggle"
            data-test={`roadmap-outcome-toggle-${outcome.id}`}
            aria-expanded={focused}
            onClick={() => onToggle(outcome.id)}
          >
            {outcome.name}
          </button>
        </h4>
        {readiness !== null && (
          <data
            className="roadmap-outcome-readiness"
            value={outcome.readinessState}
            data-test="roadmap-outcome-readiness"
          >
            {readiness}
          </data>
        )}
      </header>
      {'benefit' in outcome.why && (
        <p className="roadmap-outcome-why" data-test="roadmap-outcome-why">
          {fieldText(outcome.why.benefit)}
        </p>
      )}
      {presentMeta.length > 0 && (
        <dl className="roadmap-outcome-meta">
          {presentMeta.map(({ key, label }) => (
            <Fragment key={key}>
              <dt className="roadmap-outcome-meta-label">{label}</dt>
              <dd className="roadmap-outcome-meta-value">{fieldText(outcome.why[key])}</dd>
            </Fragment>
          ))}
        </dl>
      )}
      <footer
        className="roadmap-outcome-bands"
        role="group"
        aria-label={`Move ${outcome.name} to a horizon`}
      >
        {HORIZONS.map(({ horizon: target, label }) => (
          <button
            key={target}
            type="button"
            className="roadmap-outcome-band-pick"
            aria-pressed={target === horizon}
            disabled={target === horizon}
            onClick={() => dispatch(outcomeHorizonChangeRequested(outcome.id, target))}
          >
            {label}
          </button>
        ))}
      </footer>
      {focused && (
        <OutcomeDrillBody outcomeId={outcome.id} drill={deriveOutcomeDrillIn(outcome.iterations)} />
      )}
    </article>
  )
}

const Roadmap = (): React.JSX.Element => {
  const { bands } = useRoadmap()
  const dispatch = useDispatch()
  const [focusedId, setFocusedId] = useState<number | null>(null)
  const toggle = (outcomeId: number): void =>
    setFocusedId((current) => (current === outcomeId ? null : outcomeId))
  return (
    <section className="roadmap" data-test="roadmap">
      <p className="roadmap-live" data-test="roadmap-live">
        live · always current
      </p>
      {bands.length === 0 ? (
        <p className="roadmap-empty" data-test="roadmap-empty">
          No outcomes on the roadmap yet
        </p>
      ) : (
        bands.map((band) => (
          <section
            className="roadmap-band"
            key={band.horizon}
            data-test={`roadmap-band-${band.horizon}`}
            data-horizon={band.horizon}
          >
            <h3 className="roadmap-band-label">{band.label}</h3>
            <SortableList
              items={band.outcomes}
              keyOf={(outcome) => outcome.id}
              rowKey={(outcome) => outcome.id}
              config={OUTCOME_REPRIORITIZE_DRAG}
              buildPayload={(outcome) => outcome.id}
              onReorder={(draggedId, targetKey) => {
                const position = band.outcomes.findIndex((o) => o.id === targetKey) + 1
                if (position === 0) return
                dispatch(outcomeReprioritizeRequested(draggedId, position))
              }}
              ulClassName="roadmap-band-outcomes"
              renderRow={(outcome, { rowDragProps, gripDragProps }) => (
                <li {...rowDragProps}>
                  <RoadmapOutcomeCard
                    outcome={outcome}
                    horizon={band.horizon}
                    focused={focusedId === outcome.id}
                    onToggle={toggle}
                    gripDragProps={gripDragProps}
                  />
                </li>
              )}
            />
          </section>
        ))
      )}
    </section>
  )
}

export default Roadmap
