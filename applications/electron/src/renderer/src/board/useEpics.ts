import { useSelector } from '../transport/useSelector'
import { selectEpics } from './selectEpics'
import type { Story } from './types'

export const useEpics = (): Story[] => useSelector(selectEpics)
