// #401 + #393 AC2: runtime per-role palette.
//
// Before: `styles/tokens.css` hardcoded `--role-ryan` / `--role-dev`
// / `--role-pm` / `--role-designer` as specific hues; any role
// outside that list rendered `--role-unknown` neutral gray with
// no role-identity signal.
//
// After: a stable hash-to-hue function computes a color per role
// short-name at render time. Deterministic — same role always
// gets the same color across sessions + across refreshes —
// without requiring edits to `tokens.css` when a new agent joins.
// Renderers (avatar, @mention pill) apply the color via inline
// `style={{ color, background }}` attribute.
//
// `color-mix` via JS isn't practical in inline style; we synthesize
// the tinted background by re-using the HSL tuple with a low
// alpha channel (hsla).

const HUE_MOD = 360
const SATURATION_PCT = 70
const LIGHTNESS_PCT = 55
const BACKGROUND_ALPHA = 0.15

/**
 * Deterministic hash from a role short-name. djb2-style string
 * rolling hash — small, stable, collision-rare over short ASCII
 * role names (dev / pm / designer / ryan / ops / qa / etc).
 */
const hashRole = (role: string): number => {
  let h = 5381
  for (let i = 0; i < role.length; i++) {
    // h * 33 + charCode, forced to unsigned 32-bit so the modulus
    // at the end lands on a stable positive integer across engines.
    h = ((h << 5) + h + role.charCodeAt(i)) >>> 0
  }
  return h
}

export interface RoleColor {
  color: string
  background: string
}

/**
 * Resolve a palette entry for `role`. Same role short-name →
 * identical output across renders (stable hash). Caller applies
 * the returned `color` + `background` as inline styles on the
 * avatar / @mention pill element.
 */
export const roleColorFor = (role: string): RoleColor => {
  const hue = hashRole(role) % HUE_MOD
  return {
    color: `hsl(${hue} ${SATURATION_PCT}% ${LIGHTNESS_PCT}%)`,
    background: `hsla(${hue}, ${SATURATION_PCT}%, ${LIGHTNESS_PCT}%, ${BACKGROUND_ALPHA})`
  }
}
