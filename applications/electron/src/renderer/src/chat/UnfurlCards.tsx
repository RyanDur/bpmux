import { extractUrls, useUnfurl, type UnfurlData } from './useUnfurl'
import './UnfurlCards.css'

type Props = {
  text: string
  apiBase: string | undefined
  authToken: string | undefined
}

const hasAnyUnfurlableField = (data: UnfurlData): boolean =>
  Boolean(data.title) || Boolean(data.description) || Boolean(data.image)

const UnfurlCards = ({ text, apiBase, authToken }: Props): React.JSX.Element | null => {
  const urls = extractUrls(text)
  const unfurls = useUnfurl({ apiBase, authToken, urls })

  if (urls.length === 0) return null

  const cards: React.JSX.Element[] = []
  for (const url of urls) {
    const data = unfurls.get(url)
    if (data === undefined) {
      cards.push(
        <article
          key={url}
          className="unfurl-card unfurl-card--loading"
          data-test={`unfurl-loading-${url}`}
          aria-label={`loading preview for ${url}`}
        >
          <small className="unfurl-card-url">{url}</small>
        </article>
      )
      continue
    }
    if (!hasAnyUnfurlableField(data)) continue
    cards.push(
      <a
        key={url}
        className="unfurl-card"
        data-test={`unfurl-card-${url}`}
        href={url}
        target="_blank"
        rel="noreferrer noopener"
      >
        <article className="unfurl-card-body">
          {data.site_name && (
            <header className="unfurl-card-meta">
              <small className="unfurl-card-site">{data.site_name}</small>
            </header>
          )}
          {data.title && <strong className="unfurl-card-title">{data.title}</strong>}
          {data.description && <p className="unfurl-card-description">{data.description}</p>}
        </article>
      </a>
    )
  }

  if (cards.length === 0) return null
  return (
    <section className="unfurl-cards" data-test="unfurl-cards">
      {cards}
    </section>
  )
}

export default UnfurlCards
