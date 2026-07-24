import { useSelector } from '@state/useSelector'
import { selectEpics } from './selectEpics'
import type { Epic } from '../types'

export const useEpics = (): Epic[] => useSelector(selectEpics)
