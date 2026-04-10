# Memory Science for bpmux: Experience-to-Understanding Model

*Designer research — 2026-04-09. Directed by Ryan.*

## Key Findings

### Episodic → Semantic Memory (Tulving 1972, McClelland et al. 1995)
The log is episodic (specific events with context). The lesson index is semantic (general knowledge stripped of context). Consolidation transforms episodic into semantic — the classifier's job is exactly this: identify episodes worth extracting into semantic form. A single episode can seed a lesson, but it's fragile until reinforced.

### Memory Consolidation Signals
Three factors determine which experiences consolidate into long-term memory:
- **Emotional arousal** → correction intensity in bpmux
- **Repetition** → same lesson reinforced across conversations
- **Goal relevance** → explicit importance markers ("remember this," "always do X")

### Schema Theory (Bartlett 1932)
**Corrections are schema violations — prediction errors that trigger memory updating.** When reality violates what the brain predicted, the hippocampus shifts from maintenance to update mode. Corrections are the highest-signal experiences because they destabilize wrong schemas.

### Levels of Processing (Craik & Lockhart 1972)
**"Why" is the depth marker.** A rule ("use kebab-case") is shallow processing. A reason ("use kebab-case because the gateway normalizes casing") is deep processing. Deep processing produces durable, transferable understanding. The classifier should detect causal explanations.

### Spaced Repetition (Roediger & Karpicke 2006)
**Retrieval beats re-reading.** Actively recalling a lesson strengthens it more than re-reading. Dumping all lessons at startup (massed presentation) is the worst possible schedule. Lessons should surface when relevant to current work, not all at once.

### Transactive Memory (Wegner 1985)
**bpmux IS a transactive memory system.** Teams distribute knowledge ("who knows what") and route queries to experts. Lessons should be tagged by expertise domain. Not all peers need all lessons. Specialization reduces cognitive load.

## Practical Recommendations for bpmux

### Classifier Evolution
Detect these signals as "experiences likely to produce understanding":
- Corrections/disagreement (schema violations)
- Causal explanations with "why" (deep processing)
- Repetition of similar patterns (consolidation)
- Explicit importance markers (goal relevance)
- Surprises/novelty (prediction errors)

### Lesson Architecture
- **Dual storage:** episodic source (log excerpt) linked to semantic lesson (generalized rule)
- **Confidence levels:** new lessons start low, promote through reinforcement
- **Depth tagging:** lessons with "why" are deep, rules alone are shallow
- **Decay mechanism:** unretrieved lessons lose priority over time

### Retrieval Architecture
- **Context-triggered** instead of bulk loading — surface lessons when relevant
- **Spaced reinforcement** — resurface at expanding intervals
- **Retrieval > re-reading** — prompt recall, don't dump answers
- **Track retrieval success** — adjust confidence based on application

### Network Distribution
- **Role-based routing** — not every peer needs every lesson
- **Credibility weighting** — lessons from accurate sources carry more weight
- **Directory maintenance** — "who knows what" across the network

## Sources
Tulving (1972), McClelland et al. (1995), Bartlett (1932), Craik & Lockhart (1972), Roediger & Karpicke (2006), Wegner (1985), Sweller (1988), and others — full citations in research agent output.
