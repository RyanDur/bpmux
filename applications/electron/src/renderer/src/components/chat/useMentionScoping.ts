import { useCallback, useMemo } from 'react'

type MentionScopingApi = {
  mentionRoles: Set<string>
  handleMention: (role: string) => void
}

export const useMentionScoping = (
  presenceRoles: readonly string[],
  onSelectRecipient?: (r: string[] | null) => void
): MentionScopingApi => {
  const mentionRoles = useMemo(() => new Set(presenceRoles), [presenceRoles])
  const handleMention = useCallback(
    (role: string): void => {
      onSelectRecipient?.([role])
    },
    [onSelectRecipient]
  )
  return { mentionRoles, handleMention }
}
