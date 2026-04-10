# Graph Visualization for Arc Explorer

*Designer research — 2026-04-08.*

## Recommendation: react-force-graph-2d

Best fit for bpmux's arc explorer. Declarative React API, Canvas rendering, force-directed layout with good defaults, built-in interactions (click, hover, zoom, pan). ~150KB gzipped. Active maintenance (last release Feb 2026).

## Comparison

| Library | Bundle (gzip) | React integration | Learning curve | 100-500 nodes | Verdict |
|---|---|---|---|---|---|
| react-force-graph-2d | ~150-170KB | Excellent | Low | Good | **Pick this** |
| @react-sigma/core | ~80-100KB | Good | Medium | Overkill-good | Runner-up |
| Cytoscape.js | ~112KB | Moderate | Medium-high | Good | Over-engineered |
| vis-network | ~130-160KB | Poor | Low | Adequate | Not recommended |
| D3.js (d3-force) | ~18KB | Poor | Steep | Excellent | Too much glue code |
| Three.js / r3f | ~325KB | Excellent (3D) | Very high | Trivial | Wrong abstraction |

## Arc Explorer UX

- Nodes = arcs (sized by weight, colored by type)
- Edges = relationships (this arc builds on that arc, shared lessons)
- Click node = read the arc's messages in a side panel
- Force layout naturally clusters related arcs
- Zoom/pan for navigation
- Color-coding by classifier type when Phase 2 ships

## When to switch to Sigma.js

If the arc network exceeds 1,000 nodes (months of team learning), WebGL rendering becomes necessary. Sigma.js handles tens of thousands of nodes. Same data model — switch the renderer, keep the logic.
