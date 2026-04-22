// Per-role display label + short description. Shared constants so
// the avatar tooltip (#370), sidebar role legend (future), and any
// future role-surface all read from one place. Descriptions are
// hand-authored and aim for one-line role-purpose statements — not
// self-descriptions, not manifesto text.

export interface RoleInfo {
  label: string
  description: string
}

export const ROLE_INFO: Record<string, RoleInfo> = {
  ryan: {
    label: 'Ryan',
    description: 'owner — product direction + final acceptance'
  },
  designer: {
    label: 'designer',
    description: 'UX / accessibility / visual spec'
  },
  dev: {
    label: 'dev',
    description: 'implementation — backend + renderer + tests'
  },
  pm: {
    label: 'pm',
    description: 'product backlog + sequencing + presentation to owner'
  }
}

// Friendly-ish fallback for identities we don't recognise (guest
// peers, old role names, etc). Keeps the tooltip informative rather
// than empty when the sender isn't one of the known roles.
export const UNKNOWN_ROLE_INFO: RoleInfo = {
  label: 'unknown',
  description: 'unrecognised role or guest peer'
}

export function roleInfoFor(role: string): RoleInfo {
  return ROLE_INFO[role] ?? UNKNOWN_ROLE_INFO
}
