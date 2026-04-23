import ReactMarkdown from 'react-markdown'
import rehypeHighlight from 'rehype-highlight'
import remarkGfm from 'remark-gfm'
import 'highlight.js/styles/github-dark.css'
import './MessageBody.css'

interface Props {
  text: string
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

function MessageBody({ text }: Props): React.JSX.Element {
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
