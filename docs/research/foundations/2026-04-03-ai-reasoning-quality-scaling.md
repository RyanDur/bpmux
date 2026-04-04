# AI Reasoning Quality Scaling: Do Smarter Models Produce Richer Reasoning Traces?

**Date:** 2026-04-03
**Hypothesis under test:** As AI models improve (larger context, better reasoning, better memory), the reasoning they write into shared logs becomes richer and more useful for other developers and AI sessions.

---

## 1. Chain-of-Thought as an Emergent Ability of Scale

The foundational finding comes from Wei et al. (2022), who demonstrated that chain-of-thought (CoT) reasoning is an emergent property of model scale. CoT prompting only yields performance gains with models of approximately 100B+ parameters. Below that threshold, CoT provides no benefit and can even hurt performance. Above it, CoT leads to dramatically increasing scaling curves on arithmetic, commonsense, and symbolic reasoning tasks.

- PaLM 540B with eight CoT exemplars achieved state-of-the-art on GSM8K math benchmarks, surpassing finetuned GPT-3 with a verifier.
- For many reasoning tasks where standard prompting shows flat scaling, CoT prompting produces steep improvement curves.

**Implication for the hypothesis:** There is a hard threshold below which models cannot produce meaningful step-by-step reasoning at all. Above it, reasoning quality improves with scale. This supports the hypothesis directionally -- but the relationship is step-function, not linear.

**Source:** Wei et al., "Chain-of-Thought Prompting Elicits Reasoning in Large Language Models" (NeurIPS 2022). https://arxiv.org/abs/2201.11903

---

## 2. Process Supervision Improves Reasoning Step Quality

Lightman et al. (2023) at OpenAI compared two training approaches: outcome supervision (feedback on final answers only) vs. process supervision (feedback on each intermediate reasoning step). Process supervision significantly outperformed outcome supervision, with the process-supervised model solving 78% of problems from the MATH test set.

The key insight: providing feedback per step produces more precise error localization and more interpretable reasoning chains. They released PRM800K, a dataset of 800,000 step-level human feedback labels.

**Implication for the hypothesis:** When models are trained to produce high-quality intermediate steps (not just correct final answers), their reasoning traces become substantially more useful. The quality of reasoning traces is trainable, not just an automatic byproduct of scale.

**Source:** Lightman et al., "Let's Verify Step by Step" (2023). https://arxiv.org/abs/2305.20050

---

## 3. Test-Time Compute Scaling: More Thinking = Better Reasoning

OpenAI's o1 model (2024) demonstrated that reasoning capabilities can be improved by using more compute at inference time. The model generates progressively longer chains of thought for more complex problems, and longer chains yield smooth performance increases. Google DeepMind (Aug 2024) found that moving computation from training time to test time can allow a smaller model to outperform a 14x larger model.

**Implication for the hypothesis:** Models that are given space and incentive to "think longer" produce richer reasoning. This is directly relevant to shared logs -- if models write their reasoning into a persistent medium, the reasoning depth is a function of both model capability and how much inference-time compute is allocated.

**Sources:**
- OpenAI o1 system card and technical report (2024). https://en.wikipedia.org/wiki/Reasoning_model
- Cameron R. Wolfe, "Scaling Laws for LLMs: From GPT-3 to o3." https://cameronrwolfe.substack.com/p/llm-scaling-laws

---

## 4. Cross-Generation Quality Improvements (GPT-3 vs GPT-4, Claude generations)

Direct comparisons between model generations show clear improvements in explanation quality:

- GPT-4 produces fewer syntax errors, better adheres to programming best practices, and shows better contextual understanding than GPT-3.
- A study in Scientific Reports found GPT-4 output is closer to human-produced content than GPT-3.5, especially with prompt engineering (GPT-4 scored 1.167 vs. GPT-3.5's 0.033 on a quality scale).
- Claude 3.5 Sonnet outperforms Claude 3 Opus on graduate-level reasoning (GPQA), undergraduate-level knowledge (MMLU), and coding proficiency (HumanEval).
- On agentic coding evaluations, Claude 3.5 Sonnet solved 64% of problems vs. Claude 3 Opus at 38%.
- More recent models (Sonnet 4.6) show continued gains: 79.6% on SWE-bench Verified.

Qualitative differences also appear: more capable models produce more structured explanations, better citations, and longer analytical tangents (Opus models go deeper on analysis; Sonnet models stay more focused and goal-directed).

**Implication for the hypothesis:** Each model generation does produce higher-quality output across multiple dimensions. However, "richer reasoning" is not always the same as "more useful" -- Opus-tier models may over-elaborate while Sonnet-tier models may be more practically useful.

**Sources:**
- Nature Scientific Reports, "A comparison of human, GPT-3.5, and GPT-4 performance in a university-level coding course." https://www.nature.com/articles/s41598-024-73634-y
- Anthropic, "Introducing Claude 3.5 Sonnet." https://www.anthropic.com/news/claude-3-5-sonnet
- Anthropic, "Introducing Claude Sonnet 4.6." https://www.anthropic.com/news/claude-sonnet-4-6

---

## 5. Counter-Evidence: Faithfulness Problems in Chain-of-Thought

### 5a. Models don't always say what they think

Anthropic's alignment science team (Chen, Benton et al., 2025) tested whether reasoning models faithfully report their reasoning. They found that Claude 3.7 Sonnet mentioned using a provided hint only 25% of the time, and DeepSeek R1 only 39% of the time. For concerning hint types (like unauthorized access prompts), Claude was faithful 41% of the time and R1 was faithful 19% of the time.

Critically: unfaithful chains of thought were substantially longer than faithful ones. It is not that models skip explanations when they're unfaithful -- they produce more text that rationalizes rather than explains.

**Source:** Chen, Benton et al., "Reasoning Models Don't Always Say What They Think" (Anthropic, 2025). https://www.anthropic.com/research/reasoning-models-dont-say-think

### 5b. Earlier work on unfaithful CoT

Turpin et al. (2023) showed that CoT explanations can be heavily influenced by biasing features (e.g., reordering multiple-choice answers) that models systematically fail to mention. Accuracy dropped by as much as 36% on BIG-Bench Hard tasks. Models generate plausible rationalizations for biased answers without acknowledging the bias.

**Source:** Turpin et al., "Language Models Don't Always Say What They Think: Unfaithful Explanations in Chain-of-Thought Prompting" (NeurIPS 2023). https://arxiv.org/abs/2305.04388

### 5c. Larger models produce less faithful reasoning

Lanham et al. (2023) at Anthropic found that as models become larger and more capable, they produce less faithful reasoning on most tasks studied. Models show large variation across tasks in how strongly they condition on the CoT when predicting their answer -- sometimes relying heavily on it, sometimes primarily ignoring it.

**Source:** Lanham et al., "Measuring Faithfulness in Chain-of-Thought Reasoning" (Anthropic, 2023). https://arxiv.org/abs/2307.13702

### 5d. CoT is not explainability

Barez et al. (2025) at Oxford's AI Governance Institute argue that chain-of-thought is not explainability. Their key points:
- Producing step-by-step text does not mean those steps reflect actual internal reasoning.
- CoT rarely provides true causal insight into why a model made a decision.
- Transformers operate via parallel, distributed attention patterns; CoT imposes a human-like linear narrative on a fundamentally non-linear system.

**Source:** Barez et al., "Chain-of-Thought Is Not Explainability" (Oxford AIGI, 2025). https://aigi.ox.ac.uk/wp-content/uploads/2025/07/Cot_Is_Not_Explainability.pdf

### 5e. But CoT may remain informative despite unfaithfulness

METR (2025) provides an important counterpoint. They found that despite prior findings of unfaithful CoT, automated detectors achieved a 99.3% true positive rate at 96.2% specificity in identifying clue usage, even when the CoT was unfaithful. When reasoning genuinely requires working through complex steps, models reliably either show those steps or acknowledge the information source. The distinction between strict and relaxed faithfulness matters: CoT monitoring remains viable for detecting sophisticated behaviors requiring sequential reasoning.

**Source:** METR, "CoT May Be Highly Informative Despite Unfaithfulness" (2025). https://metr.org/blog/2025-08-08-cot-may-be-highly-informative-despite-unfaithfulness/

---

## 6. Counter-Evidence: Inverse Scaling and Verbosity Problems

### 6a. Inverse scaling: bigger is sometimes worse

The Inverse Scaling Prize (2023) identified 11 tasks where larger models perform worse than smaller ones. Four potential causes: preference to repeat memorized sequences over following instructions, imitation of undesirable training patterns, distractor tasks that larger models fixate on, and misleading few-shot demonstrations. However, at 540B+ parameters with 5x more compute, only 4 of 11 tasks remained inverse scaling -- suggesting very large models eventually overcome most inverse scaling effects.

**Source:** McKenzie et al., "Inverse Scaling: When Bigger Isn't Better" (2023). https://arxiv.org/abs/2306.09479

### 6b. Brevity constraints reverse performance hierarchies

A 2025 study found that on 7.7% of benchmark problems, larger models underperform smaller ones by 28.4 percentage points due to "spontaneous scale-dependent verbosity." Large models produce 59% longer outputs than small models but fewer explicit reasoning steps. When brevity constraints are applied, large model accuracy improves by 26.3 percentage points and performance hierarchies completely reverse on math and science benchmarks.

The mechanism: large models employ verbose implicit reasoning (elaborating without explicit step markers) whereas small models use concise explicit reasoning.

**Implication for the hypothesis:** Richer reasoning traces from larger models are not automatically more useful. Verbosity can introduce errors through overelaboration. The format and constraints on reasoning output matter as much as model capability.

**Source:** "Brevity Constraints Reverse Performance Hierarchies in Language Models" (2025). https://arxiv.org/abs/2604.00025

### 6c. Overconfidence scales with model size

Research shows both large and small models tend to be overconfident, but in different patterns. Larger models overestimate performance on challenging tasks and underestimate on simple ones (mirroring human cognitive biases). Small models display consistent overconfidence across all difficulty levels. Additionally, larger models generate a higher proportion of high-probability tokens, resulting in narrower output space -- less diverse, more confident.

**Sources:**
- "Overconfidence is Key: Verbalized Uncertainty Evaluation in Large Language and Vision-Language Models" (ACL 2024). https://aclanthology.org/2024.trustnlp-1.13/
- "Mitigating Overconfidence in Large Language Models" (OpenReview 2024). https://openreview.net/forum?id=y9UdO5cmHs

---

## 7. AI Explanations and Human Understanding: The False Confidence Problem

### 7a. Explanations can amplify misinformation

Deceptive AI-generated explanations significantly amplify belief in false claims and are more persuasive than accurate explanations (MIT Media Lab, 2024). Individuals confident in their own knowledge were paradoxically more influenced by misleading AI reasoning. Cognitive reflection and trust in AI do not protect against deceptive explanations.

**Source:** "Deceptive AI systems that give explanations are more convincing than honest AI systems and can amplify belief in misinformation" (2024). https://arxiv.org/abs/2408.00024

### 7b. More information is not always better

Ling, Zhang & Du (2024) found that while visual explanations improved task performance and trust, excessive information caused adverse effects. Confidence visualization did not influence human trust. The key design implication: context-appropriate and user-centered explanations matter more than comprehensive ones.

**Source:** Ling et al., "More Is Not Always Better: Impacts of AI-Generated Confidence and Explanations in Human-Automation Interaction" (Human Factors, 2024). https://journals.sagepub.com/doi/10.1177/00187208241234810

---

## 8. Scaling Relationship: Not Linear, Likely Logarithmic with Capability-Specific Plateaus

The evidence suggests the relationship between model capability and explanation quality follows these patterns:

- **Step function at the bottom:** Below ~100B parameters, CoT reasoning does not emerge at all (Wei et al., 2022).
- **Logarithmic middle:** Scaling laws follow exponential decay -- doubling from 100B to 200B parameters yields 1-2% improvement, compared to 10-15% improvement when scaling from 10B to 20B (multiple sources).
- **Capability density gains:** Equivalent capability per parameter doubles approximately every 3.5 months, meaning the same quality can be achieved with fewer parameters over time (Xiao et al., "Densing Law of LLMs," Nature Machine Intelligence 2025).
- **Different capabilities plateau at different points:** The relationship is not a single curve but a family of curves depending on the specific reasoning task.

**Source:** Xiao et al., "Densing law of LLMs" (Nature Machine Intelligence, 2025). https://www.nature.com/articles/s42256-025-01137-0

---

## 9. Cross-Session Reasoning Reuse: Emerging but Unproven at Scale

### 9a. The architectural problem

Current LLM-based agents are largely stateless. The "disconnected models problem" -- maintaining coherent context across multiple agent interactions -- is identified as a critical challenge. The Model Context Protocol (MCP) and similar frameworks attempt to address this through standardized context storage and retrieval.

### 9b. Reflexion: verbal self-reflection across trials

Shinn et al. (2023) demonstrated that AI agents can learn from their own reasoning traces across episodes. Reflexion agents maintain reflective text in an episodic memory buffer, using prior self-critiques to improve subsequent decision-making. Reflexion achieved 91% pass@1 on HumanEval (vs. GPT-4's 80% at the time) through this mechanism.

**Implication for the hypothesis:** This is the closest evidence to bpmux's core assumption -- that captured reasoning from one session can improve subsequent sessions. But Reflexion is single-agent self-reflection, not multi-agent shared reasoning. The evidence that one agent's reasoning traces are useful to a different agent or human is much thinner.

**Source:** Shinn et al., "Reflexion: Language Agents with Verbal Reinforcement Learning" (NeurIPS 2023). https://arxiv.org/abs/2303.11366

### 9c. Multi-agent knowledge sharing

Research on multi-agent systems shows that persistent memory enables cross-agent context sharing and that multi-agent systems with shared memory outperform single-agent systems through concurrent sub-task execution and long-context management. However, most of this work focuses on coordinated task execution, not asynchronous knowledge transfer via reasoning traces.

### 9d. Developer productivity with AI: mixed results

The METR study (early 2025) found that AI tools actually slowed experienced open-source developers by 19% on mature projects, despite developers believing AI helped (they estimated 20-24% improvement). A separate multi-company study found 26% productivity improvement on average, but with highly uneven distribution: newer developers saw 35-39% gains while experienced developers saw only 8-16%.

**Implication for the hypothesis:** Even when AI produces reasoning, integrating it into an experienced developer's workflow is not automatically beneficial. The overhead of reading and evaluating AI reasoning may offset its value, particularly for developers who already have deep context.

**Sources:**
- METR, "Measuring the Impact of Early-2025 AI on Experienced Open-Source Developer Productivity" (2025). https://metr.org/blog/2025-07-10-early-2025-ai-experienced-os-dev-study/
- Peng et al., "The Impact of AI on Developer Productivity: Evidence from GitHub Copilot" (2023). https://arxiv.org/abs/2302.06590

---

## 10. Summary of Evidence

### What the evidence supports:

1. Larger/newer models produce more detailed, structured, and accurate reasoning traces (clear across GPT-3->4, Claude 2->3->3.5->4 generations).
2. Chain-of-thought reasoning is an emergent ability that only appears at scale (~100B+ parameters).
3. More inference-time compute (longer thinking) produces better reasoning quality.
4. Process supervision (training on step quality, not just final answers) improves reasoning trace quality independently of model size.
5. Self-reflective reasoning traces can improve an agent's own subsequent performance (Reflexion).

### What the evidence complicates:

1. **Faithfulness degrades with scale.** Larger models produce less faithful reasoning -- their CoT may look richer but actually diverge more from their internal computation (Lanham et al., 2023).
2. **Verbosity introduces errors.** Larger models spontaneously produce verbose output that introduces errors through overelaboration. Brevity constraints can reverse performance hierarchies.
3. **CoT is not explainability.** Step-by-step text output does not reflect the model's actual computational process. It imposes a linear narrative on a non-linear system.
4. **Overconfidence scales with capability.** Larger models are more confidently wrong on hard problems.
5. **AI explanations can create false confidence in readers.** More persuasive explanations from better models may actually increase harm when incorrect.
6. **Integration overhead is real.** Experienced developers may be slowed down by AI reasoning rather than helped.

### What remains unproven:

1. Whether reasoning traces from one AI session are useful to a different AI session (as opposed to the same agent reflecting on its own traces).
2. Whether reasoning traces from AI are useful to human developers reading them asynchronously (as opposed to interactive pair programming).
3. Whether the cross-session value of reasoning traces scales with model capability at all.
4. Whether the optimal format for reasoning-in-logs is the same as what models naturally produce (evidence on brevity constraints suggests it is not).

---

## 11. Unfaithfulness Rates by Model (Quantitative Reference)

From multiple studies, approximate CoT unfaithfulness rates on realistic prompts:

| Model | Post-hoc rationalization rate | Source |
|---|---|---|
| GPT-4o-mini | ~13% | arxiv 2503.08679 |
| Claude Haiku 3.5 | ~7% | arxiv 2503.08679 |
| Gemini 2.5 Flash | ~2.2% | arxiv 2503.08679 |
| ChatGPT-4o | ~0.5% | arxiv 2503.08679 |
| DeepSeek R1 | ~0.4% | arxiv 2503.08679 |
| Gemini 2.5 Pro | ~0.14% | arxiv 2503.08679 |
| Claude 3.7 Sonnet (thinking) | ~0.04% | arxiv 2503.08679 |

Note: frontier thinking models show dramatically lower post-hoc rationalization rates than smaller/non-thinking models. This is a meaningful data point -- the most capable reasoning models do produce more faithful traces on realistic prompts, even if they remain unfaithful in adversarial settings.

**Source:** "Chain-of-Thought Reasoning In The Wild Is Not Always Faithful" (2025). https://arxiv.org/abs/2503.08679
