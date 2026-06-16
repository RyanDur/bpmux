import { Fragment, useMemo, useState } from 'react'
import { deriveRoadmapView, type OutcomeWhy, type RoadmapOutcome } from './deriveRoadmapView'
import { deriveOutcomeDrillIn } from './deriveOutcomeDrillIn'
import OutcomeDrillBody from './OutcomeDrillBody'
import type { BoardStory } from './types'
import './Roadmap.css'

interface RoadmapProps {
  stories: BoardStory[]
}

const META_FIELDS: ReadonlyArray<{ key: keyof OutcomeWhy; label: string }> = [
  { key: 'metric', label: 'Metric' },
  { key: 'confidence', label: 'Confidence' },
  { key: 'value', label: 'Value' }
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
  stories,
  focused,
  onToggle
}: {
  outcome: RoadmapOutcome
  stories: BoardStory[]
  focused: boolean
  onToggle: (outcomeNumber: number) => void
}): React.JSX.Element => {
  const readiness = readinessText(outcome)
  const presentMeta = META_FIELDS.filter(({ key }) => key in outcome.why)
  return (
    <article
      className="roadmap-outcome"
      data-test={`roadmap-outcome-${outcome.number}`}
      data-tone={outcome.tone}
      data-focused={focused || undefined}
    >
      <header className="roadmap-outcome-head">
        <h4 className="roadmap-outcome-name">
          <button
            type="button"
            className="roadmap-outcome-toggle"
            data-test={`roadmap-outcome-toggle-${outcome.number}`}
            aria-expanded={focused}
            onClick={() => onToggle(outcome.number)}
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
      {focused && (
        <OutcomeDrillBody
          outcomeNumber={outcome.number}
          drill={deriveOutcomeDrillIn(outcome.number, stories)}
        />
      )}
    </article>
  )
}

const Roadmap = ({ stories }: RoadmapProps): React.JSX.Element => {
  const bands = useMemo(() => deriveRoadmapView(stories), [stories])
  const [focusedNumber, setFocusedNumber] = useState<number | null>(null)
  const toggle = (outcomeNumber: number): void =>
    setFocusedNumber((current) => (current === outcomeNumber ? null : outcomeNumber))
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
            <ul className="roadmap-band-outcomes">
              {band.outcomes.map((outcome) => (
                <li key={outcome.number}>
                  <RoadmapOutcomeCard
                    outcome={outcome}
                    stories={stories}
                    focused={focusedNumber === outcome.number}
                    onToggle={toggle}
                  />
                </li>
              ))}
            </ul>
          </section>
        ))
      )}
    </section>
  )
}

export default Roadmap
