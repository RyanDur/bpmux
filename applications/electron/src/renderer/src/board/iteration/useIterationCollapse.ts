import { useCollapseMap, type CollapseMap } from '../useCollapseMap'

export type IterationCollapseStorage = CollapseMap

export const useIterationCollapse = (viewerIdentity: string): IterationCollapseStorage =>
  useCollapseMap('bpmux-iteration-collapse', viewerIdentity)
