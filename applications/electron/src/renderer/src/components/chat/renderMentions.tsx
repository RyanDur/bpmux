import type { ReactNode } from 'react'
import { roleColorFor } from '../roles/roleColor'

const MENTION_RE = /(^|[^A-Za-z0-9_@])@([A-Za-z0-9_-]+)/g

const splitMentions = (
  text: string,
  knownRoles: ReadonlySet<string>,
  onMention: ((role: string) => void) | undefined,
  keyPrefix: string
): ReactNode[] => {
  if (knownRoles.size === 0) return [text]
  const out: ReactNode[] = []
  let lastIndex = 0
  let i = 0
  for (const match of text.matchAll(MENTION_RE)) {
    const [, leading, role] = match
    const start = (match.index ?? 0) + leading.length
    const end = start + 1 + role.length
    if (!knownRoles.has(role)) continue
    if (start > lastIndex) {
      out.push(text.slice(lastIndex, start))
    }
    const key = `${keyPrefix}:mention:${i++}:${start}`
    const handleClick = (): void => {
      onMention?.(role)
    }
    const pillColor = roleColorFor(role)
    out.push(
      onMention ? (
        <button
          key={key}
          type="button"
          className="message-mention"
          data-role={role}
          data-test="message-mention"
          style={{ color: pillColor.color, background: pillColor.background }}
          onClick={handleClick}
        >
          @{role}
        </button>
      ) : (
        <b
          key={key}
          className="message-mention"
          data-role={role}
          data-test="message-mention"
          style={{ color: pillColor.color, background: pillColor.background }}
        >
          @{role}
        </b>
      )
    )
    lastIndex = end
  }
  if (lastIndex < text.length) {
    out.push(text.slice(lastIndex))
  }
  return out.length === 0 ? [text] : out
}

export const replaceMentionsInChildren = (
  children: ReactNode,
  knownRoles: ReadonlySet<string>,
  onMention: ((role: string) => void) | undefined,
  keyPrefix: string
): ReactNode => {
  if (!Array.isArray(children)) {
    if (typeof children === 'string') {
      return splitMentions(children, knownRoles, onMention, keyPrefix)
    }
    return children
  }
  return children.flatMap((child, idx) => {
    if (typeof child === 'string') {
      return splitMentions(child, knownRoles, onMention, `${keyPrefix}:${idx}`)
    }
    return [child]
  })
}
