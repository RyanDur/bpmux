import { useSelector } from '../transport/useSelector'
import { selectIterations, type BoardIterationData } from './selectIterations'

export const useIterations = (): BoardIterationData[] => useSelector(selectIterations)
