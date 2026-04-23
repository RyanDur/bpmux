import { useId } from 'react'
import ReactMarkdown from 'react-markdown'
import rehypeHighlight from 'rehype-highlight'
import remarkGfm from 'remark-gfm'
import 'highlight.js/styles/github-dark.css'
import { replaceMentionsInChildren } from './renderMentions'
import './MessageBody.css'

interface Props {
  text: string
  // #380: when provided, inline @role strings matching any entry
  // of `knownRoles` render as `.message-mention` pills. Omit (or
  // pass an empty set) to get plain-text rendering — the default
  // for surfaces like the drill-in story body + comments where
  // @-pill treatment isn't part of the feature.
  knownRoles?: ReadonlySet<string>
  // Click handler for a mention pill. When provided, the pill
  // renders as a button (role/tab-index/Enter+Space activation).
  // Omit to render pill as decorative-only.
  onMention?: (role: string) => void
}

// #366: a `#comment-<id>` fragment inside a quote-block anchor
// points back to the original comment in a drill-in. When we see one
// we scroll that comment into view and re-use the existing
// `--highlight` class (100ms fade-in / 1.5s fade-out via the CSS in
// DrillIn.css — same affordance #344 search-navigation uses, kept
// consistent so the two navigation paths feel identical). If the
// drill-in isn't open / the element isn't in the DOM, querySelector
// returns null and we no-op — graceful fallback.
const COMMENT_ANCHOR_RE = /^#comment-(\d+)$/
const FLASH_CLASS = 'drill-in-comment--highlight'
const FLASH_MS = 1500

function handleCommentAnchor(id: string): void {
  const el = document.querySelector(`[data-comment-id="${id}"]`)
  if (!(el instanceof HTMLElement)) return
  el.scrollIntoView({ behavior: 'smooth', block: 'center' })
  el.classList.add(FLASH_CLASS)
  window.setTimeout(() => el.classList.remove(FLASH_CLASS), FLASH_MS)
}

function MessageBody({ text, knownRoles, onMention }: Props): React.JSX.Element {
  // Stable per-render key prefix so mention spans within a given
  // message don't collide with their siblings across re-renders.
  const mentionKeyPrefix = useId()
  const mentionRoles = knownRoles ?? new Set<string>()
  const withMentions = (children: React.ReactNode): React.ReactNode =>
    replaceMentionsInChildren(children, mentionRoles, onMention, mentionKeyPrefix)

  return (
    // #381: `.markdown-body` scope restores Meyer-reset-stripped
    // defaults on inline (strong / em / code) + block (h1-h6, ul, ol,
    // blockquote) elements via `revert`. Applied via MessageBody.css
    // so every consumer (chat message, drill-in story body, drill-
    // in comment body) inherits the restoration uniformly.
    <div className="markdown-body">
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        rehypePlugins={[rehypeHighlight]}
        components={{
          // #380: override text-containing inline + block elements
          // so their string children get scanned for @-mentions.
          // Unknown-role @-tokens pass through unchanged (AC2).
          p: ({ children }) => <p>{withMentions(children)}</p>,
          li: ({ children }) => <li>{withMentions(children)}</li>,
          strong: ({ children }) => <strong>{withMentions(children)}</strong>,
          em: ({ children }) => <em>{withMentions(children)}</em>,
          a: ({ href, children }) => (
            <a
              href={href}
              onClick={(e) => {
                e.preventDefault()
                if (!href) return
                const match = COMMENT_ANCHOR_RE.exec(href)
                if (match) {
                  handleCommentAnchor(match[1])
                  return
                }
                window.open(href, '_blank', 'noopener,noreferrer')
              }}
            >
              {children}
            </a>
          )
        }}
      >
        {text}
      </ReactMarkdown>
    </div>
  )
}

export default MessageBody
