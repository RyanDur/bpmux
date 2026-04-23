import type { ReactNode } from 'react'
import { roleColorFor } from '../roles/roleColor'

// #380 inline @mention pills in message body text.
//
// react-markdown renders text as React strings; there's no
// top-level "text" component hook in v9 to intercept. Instead,
// consumers override container elements (p / li / etc) and walk
// their children, passing any string to `splitMentions` to expand
// @-matches into pill spans.
//
// Role validity comes from a caller-provided set — per #393 AC1
// discipline, the renderer doesn't hardcode team composition.
// Pass the output of `deriveTeamRoles(messages, viewer)` when you
// want @-pill rendering; omit / pass undefined to disable (plain
// text render, matches per-AC2 "unknown @foo stays plain text").

// Matches `@name` where `name` is a word-ish token. The regex is
// anchored to word-boundary at the start so "foo@bar" doesn't
// match, but whitespace-prefixed + leading-of-string both do.
const MENTION_RE = /(^|[^A-Za-z0-9_@])@([A-Za-z0-9_-]+)/g

/**
 * Split a string into alternating plain-text and mention-pill
 * ReactNode segments. Mentions matching roles outside `knownRoles`
 * pass through unchanged (plain text, per #380 AC2).
 *
 * `keyPrefix` is used to build stable React keys; pass a unique
 * prefix (e.g., message id) per container so adjacent paragraphs'
 * mentions don't collide under reconciliation.
 */
export function splitMentions(
  text: string,
  knownRoles: ReadonlySet<string>,
  onMention: ((role: string) => void) | undefined,
  keyPrefix: string
): ReactNode[] {
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
      // Include the leading char + any text before it.
      out.push(text.slice(lastIndex, start))
    }
    const key = `${keyPrefix}:mention:${i++}:${start}`
    const handleClick = (): void => {
      onMention?.(role)
    }
    // #401: pill color comes from the runtime `roleColorFor` palette
    // so any role (ryan/dev/pm/designer AND future ops/qa/...) gets a
    // distinct identity-tint without a per-role CSS rule.
    const pillColor = roleColorFor(role)
    out.push(
      <span
        key={key}
        className="message-mention"
        data-role={role}
        data-test="message-mention"
        style={{ color: pillColor.color, background: pillColor.background }}
        role={onMention ? 'button' : undefined}
        tabIndex={onMention ? 0 : undefined}
        onClick={onMention ? handleClick : undefined}
        onKeyDown={
          onMention
            ? (e) => {
                if (e.key === 'Enter' || e.key === ' ') {
                  e.preventDefault()
                  handleClick()
                }
              }
            : undefined
        }
      >
        @{role}
      </span>
    )
    lastIndex = end
  }
  if (lastIndex < text.length) {
    out.push(text.slice(lastIndex))
  }
  return out.length === 0 ? [text] : out
}

/**
 * Walk React children; when a child is a string, replace it with
 * the splitMentions output. Non-string children (inline markup
 * from react-markdown — strong / em / code / etc) pass through.
 * The walk is shallow — a mention inside <strong>@pm</strong>
 * stays inside the strong because that's its natural scope per
 * markdown semantics.
 */
export function replaceMentionsInChildren(
  children: ReactNode,
  knownRoles: ReadonlySet<string>,
  onMention: ((role: string) => void) | undefined,
  keyPrefix: string
): ReactNode {
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
