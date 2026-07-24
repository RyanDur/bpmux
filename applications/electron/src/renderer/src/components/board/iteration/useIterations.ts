import { useSelector } from '@state/useSelector'
import { selectIterations, type BoardIterationData } from './selectIterations'

export const useIterations = (): BoardIterationData[] => useSelector(selectIterations)
