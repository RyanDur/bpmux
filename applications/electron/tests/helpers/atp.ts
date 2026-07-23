import { basename } from 'path'

export const atpPath = (specFile: string): string =>
  `applications/electron/tests/${basename(specFile)}`
