import { Terminal } from '@xterm/xterm'

export const copyOnCmdC = (term: Terminal, event: KeyboardEvent): boolean => {
  if (event.type === 'keydown' && event.metaKey && !event.ctrlKey && event.key === 'c') {
    const selection = term.getSelection()
    if (selection.length > 0) {
      void navigator.clipboard.writeText(selection)
      return false
    }
  }
  return true
}
