export enum PtyActionType {
  PtyOutputReceived = 'state/ptyOutputReceived',
  PtyExitReceived = 'state/ptyExitReceived',
  PtyModeReceived = 'state/ptyModeReceived',
  PtySubscribeRequested = 'state/ptySubscribeRequested',
  PtyInputRequested = 'state/ptyInputRequested',
  PtyResizeRequested = 'state/ptyResizeRequested'
}

export type PtyAction =
  | { type: PtyActionType.PtyOutputReceived; terminalId: string; data: string }
  | { type: PtyActionType.PtyExitReceived; terminalId: string }
  | { type: PtyActionType.PtyModeReceived; terminalId: string; bracketedPaste: boolean }
  | { type: PtyActionType.PtySubscribeRequested; terminalId: string }
  | { type: PtyActionType.PtyInputRequested; terminalId: string; keystroke: string }
  | { type: PtyActionType.PtyResizeRequested; terminalId: string; cols: number; rows: number }

export const ptyOutputReceived = (terminalId: string, data: string): PtyAction => ({
  type: PtyActionType.PtyOutputReceived,
  terminalId,
  data
})

export const ptyExitReceived = (terminalId: string): PtyAction => ({
  type: PtyActionType.PtyExitReceived,
  terminalId
})

export const ptyModeReceived = (terminalId: string, bracketedPaste: boolean): PtyAction => ({
  type: PtyActionType.PtyModeReceived,
  terminalId,
  bracketedPaste
})

export const ptySubscribeRequested = (terminalId: string): PtyAction => ({
  type: PtyActionType.PtySubscribeRequested,
  terminalId
})

export const ptyInputRequested = (terminalId: string, keystroke: string): PtyAction => ({
  type: PtyActionType.PtyInputRequested,
  terminalId,
  keystroke
})

export const ptyResizeRequested = (terminalId: string, cols: number, rows: number): PtyAction => ({
  type: PtyActionType.PtyResizeRequested,
  terminalId,
  cols,
  rows
})
