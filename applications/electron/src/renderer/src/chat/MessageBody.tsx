import ReactMarkdown from 'react-markdown'
import rehypeHighlight from 'rehype-highlight'
import remarkGfm from 'remark-gfm'
import 'highlight.js/styles/github-dark.css'

interface Props {
  text: string
}

function MessageBody({ text }: Props): React.JSX.Element {
  return (
    <ReactMarkdown
      remarkPlugins={[remarkGfm]}
      rehypePlugins={[rehypeHighlight]}
      components={{
        a: ({ href, children }) => (
          <a
            href={href}
            onClick={(e) => {
              e.preventDefault()
              if (href) window.open(href, '_blank', 'noopener,noreferrer')
            }}
          >
            {children}
          </a>
        )
      }}
    >
      {text}
    </ReactMarkdown>
  )
}

export default MessageBody
