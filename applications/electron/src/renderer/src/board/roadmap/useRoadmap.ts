import { useSelector } from '../../state/useSelector'
import { selectRoadmap, type Roadmap } from './selectRoadmap'
export const useRoadmap = (): Roadmap => useSelector(selectRoadmap)
