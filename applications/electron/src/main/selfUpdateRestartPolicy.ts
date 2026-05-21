export interface SelfUpdateRestartPolicy {
  windowMs: number
  maxRestarts: number
}

export type SelfUpdateRestartDecision =
  | { action: 'respawn'; restartsInWindow: number }
  | { action: 'halt'; restartsInWindow: number }

export const applySelfUpdateRestartPolicy = (
  recentAt: number[],
  now: number,
  policy: SelfUpdateRestartPolicy
): SelfUpdateRestartDecision => {
  while (recentAt.length > 0 && now - recentAt[0]! > policy.windowMs) {
    recentAt.shift()
  }
  recentAt.push(now)
  if (recentAt.length >= policy.maxRestarts) {
    return { action: 'halt', restartsInWindow: recentAt.length }
  }
  return { action: 'respawn', restartsInWindow: recentAt.length }
}
