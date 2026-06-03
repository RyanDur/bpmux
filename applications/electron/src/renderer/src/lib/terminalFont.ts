const NERD_FONT = '"MesloLGS NF"'
const TERMINAL_MONO_FALLBACK = 'ui-monospace, SFMono-Regular, Menlo, monospace'

export const composeTerminalFontFamily = (userMono: string): string =>
  [userMono, NERD_FONT, TERMINAL_MONO_FALLBACK].filter((part) => part.length > 0).join(', ')
