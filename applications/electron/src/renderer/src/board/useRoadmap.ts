import { useSelector } from '../transport/useSelector'
import { selectRoadmap, type Roadmap } from './selectRoadmap'
export const useRoadmap = (): Roadmap => useSelector(selectRoadmap)
