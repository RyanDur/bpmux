import { useId } from 'react'
import ReactMarkdown from 'react-markdown'
import rehypeHighlight from 'rehype-highlight'
import remarkGfm from 'remark-gfm'
import 'highlight.js/styles/github-dark.css'
import { replaceMentionsInChildren } from './renderMentions'
import './MessageBody.css'

interface Props {
  text: string
  knownRoles?: ReadonlySet<string>
  onMention?: (role: string) => void
}

const COMMENT_ANCHOR_RE = /^#comment-(\d+)$/
const FLASH_CLASS = 'drill-in-comment--highlight'
const FLASH_MS = 1500

const handleCommentAnchor = (id: string): void => {
  const el = document.querySelector(`[data-comment-id="${id}"]`)
  if (!(el instanceof HTMLElement)) return
  el.scrollIntoView({ behavior: 'smooth', block: 'center' })
  el.classList.add(FLASH_CLASS)
  window.setTimeout(() => el.classList.remove(FLASH_CLASS), FLASH_MS)
}

const MessageBody = ({ text, knownRoles, onMention }: Props): React.JSX.Element => {
  const mentionKeyPrefix = useId()
  const mentionRoles = knownRoles ?? new Set<string>()
  const withMentions = (children: React.ReactNode): React.ReactNode =>
    replaceMentionsInChildren(children, mentionRoles, onMention, mentionKeyPrefix)

  return (
    <article className="markdown-body">
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        rehypePlugins={[rehypeHighlight]}
        components={{
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
    </article>
  )
}

export default MessageBody
