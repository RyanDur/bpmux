const plumbing =
  /^(ingest: |source unreachable: |fetch_[a-z_]+\([^)]*\): |refresh_story\([^)]*\): )/

export const presentSyncError = (raw: string): string => {
  let message = raw
  while (plumbing.test(message)) {
    message = message.replace(plumbing, '')
  }
  return message
}
