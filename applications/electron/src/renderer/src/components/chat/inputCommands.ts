const VALID_ADDRESSEES = new Set(['owner', 'dev', 'pm', 'designer'])

export const parseAddressee = (text: string): string | null => {
  const match = text.match(/^@(\w+)\b/)
  if (!match) return null
  const role = match[1].toLowerCase()
  return VALID_ADDRESSEES.has(role) ? role : null
}

const isWordChar = (ch: string | undefined): boolean => {
  if (!ch) return false
  return /[\w]/.test(ch)
}

export const prevWordBoundary = (value: string, from: number): number => {
  let i = from
  while (i > 0 && !isWordChar(value[i - 1])) i -= 1
  while (i > 0 && isWordChar(value[i - 1])) i -= 1
  return i
}

export const nextWordBoundary = (value: string, from: number): number => {
  let i = from
  while (i < value.length && !isWordChar(value[i])) i += 1
  while (i < value.length && isWordChar(value[i])) i += 1
  return i
}

export const lineStart = (value: string, from: number): number => {
  const nl = value.lastIndexOf('\n', from - 1)
  return nl === -1 ? 0 : nl + 1
}

export const lineEnd = (value: string, from: number): number => {
  const nl = value.indexOf('\n', from)
  return nl === -1 ? value.length : nl
}

export const setCaret = (ta: HTMLTextAreaElement, pos: number): void => {
  ta.selectionStart = pos
  ta.selectionEnd = pos
}

export const copyToClipboard = (text: string): void => {
  if (!text) return
  if (typeof navigator === 'undefined' || !navigator.clipboard) return
  void navigator.clipboard.writeText(text).catch(() => {
    return
  })
}
