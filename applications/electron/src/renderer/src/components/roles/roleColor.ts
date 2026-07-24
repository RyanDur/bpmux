const HUE_MOD = 360
const SATURATION_PCT = 70
const LIGHTNESS_PCT = 55
const BACKGROUND_ALPHA = 0.15

const hashRole = (role: string): number => {
  let h = 5381
  for (let i = 0; i < role.length; i++) {
    h = ((h << 5) + h + role.charCodeAt(i)) >>> 0
  }
  return h
}

export type RoleColor = {
  color: string
  background: string
}

export const roleColorFor = (role: string): RoleColor => {
  const hue = hashRole(role) % HUE_MOD
  return {
    color: `hsl(${hue} ${SATURATION_PCT}% ${LIGHTNESS_PCT}%)`,
    background: `hsla(${hue}, ${SATURATION_PCT}%, ${LIGHTNESS_PCT}%, ${BACKGROUND_ALPHA})`
  }
}
