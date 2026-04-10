# Text Classification Techniques for Chat Message Categorization

**Date:** 2026-04-08
**Context:** bpmux chat log (~15K messages) needs classification into
categories like corrections, discussions, decisions, questions, action
items, etc. Classifier must be lightweight, fast, and runnable locally
without API calls.

---

## 1. Traditional ML Classifiers

These are well-understood, fast at inference, and have minimal
dependencies. They require labeled training data (typically 100-1000+
examples per class).

### 1.1 Naive Bayes (Multinomial / Complement)

**How it works:** Treats each document as a bag of words and applies
Bayes' theorem. Multinomial NB uses word counts; Complement NB adjusts
for class imbalance by using complement class statistics.

- **Strengths:** Extremely fast training and inference. Works well with
  small datasets (50-200 examples per class can suffice). Simple to
  implement. Low memory footprint.
- **Weaknesses:** Assumes feature independence (words are independent),
  which is wrong but often works anyway. Struggles with nuanced
  categories where word overlap is high. Poor at capturing word order
  or context.
- **Data requirements:** 50-500 labeled examples per class. More helps
  but diminishing returns set in quickly.
- **When to use:** First baseline. If Naive Bayes gets 80%+ accuracy,
  the problem is well-separated and you may not need anything fancier.
  Also good when categories are defined by distinctive vocabulary
  (e.g., "decision" messages contain words like "agreed", "resolved",
  "let's go with").
- **Rust feasibility:** Trivially implementable from scratch or via the
  `linfa-bayes` crate.

### 1.2 SVM (Linear, with TF-IDF features)

**How it works:** Maps TF-IDF feature vectors into high-dimensional
space and finds the hyperplane that best separates classes. Linear SVM
is the standard for text classification since Joachims (1998).

- **Strengths:** Best-in-class among traditional ML for text. Handles
  high-dimensional sparse features (TF-IDF) naturally. Excellent
  generalization even with moderate data. Fast inference.
- **Weaknesses:** Training can be slow on very large datasets (not an
  issue at 15K). Requires feature engineering (TF-IDF parameters,
  n-gram choices). Binary by nature; multi-class requires one-vs-rest
  or one-vs-one.
- **Data requirements:** 200-1000 labeled examples per class for good
  performance. Benefits from more data more than Naive Bayes does.
- **When to use:** When you need better accuracy than Naive Bayes and
  have enough labeled data. The go-to traditional classifier for text
  before 2018.
- **Rust feasibility:** `linfa-svm` crate exists. TF-IDF requires
  manual implementation or a tokenizer crate, but is straightforward.

### 1.3 Logistic Regression

**How it works:** Linear model with softmax for multi-class. Similar
to linear SVM in practice for text classification.

- **Strengths:** Produces calibrated probabilities (useful for "how
  confident is this classification?"). Fast. Well-understood
  regularization (L1 for feature selection, L2 for generalization).
- **Weaknesses:** Similar accuracy to linear SVM (sometimes slightly
  worse). Same feature engineering requirements.
- **Data requirements:** Same as SVM, 200-1000 per class.
- **When to use:** When you need confidence scores, not just labels.
  Useful for flagging low-confidence predictions for human review.
- **Rust feasibility:** `linfa-logistic` crate. Straightforward.

### 1.4 Random Forest / Gradient Boosting (XGBoost, LightGBM)

**How it works:** Ensemble of decision trees. Random Forest trains
trees in parallel on random subsets; gradient boosting trains
sequentially, each tree correcting errors of the previous.

- **Strengths:** Handles mixed feature types well (combine TF-IDF with
  metadata like message length, time of day, presence of @mentions).
  Less sensitive to hyperparameter tuning than SVM.
- **Weaknesses:** Generally worse than linear SVM on pure text
  features. Models are larger. Gradient boosting is slower to train.
- **Data requirements:** 500+ per class to beat simpler methods.
- **When to use:** When you have rich metadata features alongside text
  (message length, position in conversation, author role, time gap
  from previous message). The metadata features are where these shine.
- **Rust feasibility:** `linfa-trees` exists but is basic. No mature
  Rust XGBoost equivalent. Could call the C XGBoost library via FFI,
  but adds complexity.

### Traditional ML Summary

| Method              | Accuracy (typical) | Training data needed | Inference speed | Rust maturity |
|---------------------|--------------------|----------------------|-----------------|---------------|
| Naive Bayes         | 70-82%             | 50-500/class         | <1ms/msg        | Good          |
| Linear SVM          | 78-88%             | 200-1000/class       | <1ms/msg        | Good          |
| Logistic Regression | 76-86%             | 200-1000/class       | <1ms/msg        | Good          |
| Gradient Boosting   | 80-90%             | 500+/class           | 1-5ms/msg       | Limited       |

---

## 2. Embedding-Based Classification (Post-LLM Era)

The shift since 2020: instead of hand-crafted TF-IDF features, use
pre-trained neural networks to produce dense vector representations
(embeddings) that capture semantic meaning.

### 2.1 Sentence Transformers

**Key models:**
- **all-MiniLM-L6-v2** (22M params, 80MB): The workhorse. Fast, good
  quality, small. Produces 384-dimensional embeddings. Runs on CPU in
  ~5ms per sentence.
- **all-mpnet-base-v2** (110M params, 420MB): Higher quality, slower.
  768-dimensional embeddings. ~20ms per sentence on CPU.
- **BGE-small-en-v1.5** (33M params, 130MB): Strong alternative from
  BAAI. Good balance of size and quality.
- **GTE-small** (33M params): Another competitive small model.
- **Nomic-embed-text-v1.5** (137M params): Newer, Matryoshka
  embeddings (can truncate dimensions for speed/size tradeoff).

**How classification works with embeddings:**
1. Embed all labeled examples.
2. For a new message, embed it and find the nearest labeled examples
   (k-NN with cosine similarity).
3. Or: train a simple classifier (logistic regression, small MLP) on
   top of the embeddings instead of TF-IDF features.

### 2.2 How Embeddings Changed Classification

**Before (pre-2020):** Feature engineering was the bottleneck. TF-IDF
misses synonyms, paraphrases, and context. "Let's go with option A"
and "I vote for the first approach" look very different to TF-IDF but
mean the same thing.

**After:** Embeddings capture semantic similarity. Classification
accuracy improved 5-15% on most benchmarks without any feature
engineering. The trade-off is that you need a model to produce
embeddings (typically 20-400MB), whereas TF-IDF is just arithmetic on
word counts.

### 2.3 Local Embedding Models

All sentence transformer models can run locally. No API calls needed.

- **ONNX Runtime:** Export any sentence transformer to ONNX format.
  Inference is 2-5x faster than PyTorch, and ONNX Runtime has no
  Python dependency (C/C++/Rust bindings available).
- **Candle (Rust):** Hugging Face's Rust ML framework can load and run
  sentence transformer models natively. See section 5.3.
- **llamafile / GGML:** Some embedding models are available in
  quantized GGML format, further reducing size and improving speed.

### 2.4 Practical Performance

For 15K messages with all-MiniLM-L6-v2:
- Embedding all 15K messages: ~75 seconds on CPU (single-threaded),
  ~15 seconds with batching and parallelism.
- One-time cost; store embeddings for reuse.
- Single message embedding: ~5ms. Fast enough for real-time
  classification.
- Memory: ~23MB for the model + ~23MB for 15K x 384-dim float32
  vectors.

---

## 3. Few-Shot and Zero-Shot Classification

These approaches reduce or eliminate the need for labeled training
data, which is the primary bottleneck in classification projects.

### 3.1 SetFit (Few-Shot)

**How it works:** Fine-tunes a sentence transformer using contrastive
learning on just 8-64 labeled examples per class. Pairs of same-class
examples are pulled together in embedding space; different-class pairs
are pushed apart. Then trains a small classification head.

- **Strengths:** 8-16 examples per class often matches fully
  supervised models trained on hundreds. No prompts or language model
  needed. Fine-tuning takes seconds to minutes.
- **Weaknesses:** Requires a sentence transformer as the base (adds
  model size). Quality depends on the base model. Still needs _some_
  labeled data.
- **Data requirements:** 8-64 labeled examples per class. This is the
  breakthrough: you can hand-label 50-100 messages total and have a
  usable classifier.
- **Practical for bpmux:** Very practical. Label 10 examples each of
  corrections, discussions, decisions, etc. Fine-tune. Iterate.
- **Rust feasibility:** The fine-tuning step is Python-only (for now),
  but the resulting model can be exported to ONNX and run in Rust.

### 3.2 Zero-Shot Classification with NLI Models

**How it works:** Uses a Natural Language Inference model (trained on
premise/hypothesis pairs) to classify without any labeled examples.
You provide candidate labels as hypotheses: "This message is a
decision" / "This message is a correction" / etc. The NLI model scores
each hypothesis against the message.

**Key models:**
- **bart-large-mnli** (400M params): The classic. Too large for CLI
  use.
- **deberta-v3-base-mnli** (86M params): Smaller, competitive.
- **MoritzLaurer/deberta-v3-base-zeroshot-v2.0** (184M params):
  Specifically trained for zero-shot classification. Best quality in
  this size range.

- **Strengths:** Zero labeled data required. Just define your category
  names and go. Easy to iterate on categories.
- **Weaknesses:** Slower (one inference per candidate label per
  message). Lower accuracy than few-shot or supervised approaches.
  Models are larger (86-400MB). Less reliable on ambiguous categories.
- **Data requirements:** Zero labeled data. Just label names and
  optional descriptions.
- **Practical for bpmux:** Good for prototyping and discovering what
  categories naturally exist. Not great as a production classifier
  due to speed and accuracy concerns.
- **Rust feasibility:** Models can be exported to ONNX. Inference in
  Rust via `ort` crate. But the models are large.

### 3.3 When to Use Which

| Approach       | Labeled data | Accuracy | Speed      | Model size |
|----------------|-------------|----------|------------|------------|
| Zero-shot NLI  | 0           | 65-78%   | 50-200ms   | 86-400MB   |
| SetFit          | 8-64/class  | 80-90%   | 5-20ms     | 80-420MB   |
| Supervised (embeddings + LR) | 200+/class | 85-92% | 5-20ms | 80-420MB |
| Supervised (TF-IDF + SVM)    | 200+/class | 78-88% | <1ms   | <10MB    |

---

## 4. Semantic Search

Semantic search is closely related to classification and is directly
relevant to finding "arcs" (thematic threads) in conversation.

### 4.1 Vector Databases

**What they do:** Store embeddings and enable fast nearest-neighbor
search over them.

- **FAISS** (Facebook AI Similarity Search): The gold standard for
  local vector search. C++ with Python bindings. Supports flat
  (exact) and approximate (IVF, HNSW) indices. At 15K vectors, flat
  search is fast enough (<5ms). No server process needed.
- **Qdrant:** Rust-native vector database. Can run embedded (in-process)
  or as a server. Rich filtering. Overkill for 15K vectors but the
  Rust-native aspect is appealing.
- **ChromaDB:** Python-native, simple API. Good for prototyping. Not
  useful for Rust.
- **lancedb:** Rust-friendly, embedded, columnar. Newer but promising.
- **usearch (USearch):** Compact single-header C++ library with Rust
  bindings. Very fast HNSW implementation. Ideal for embedding into
  CLI tools.

**For 15K vectors:** A flat index (brute-force cosine similarity) is
likely sufficient. HNSW adds complexity for marginal speed gains at
this scale. At 100K+ vectors, approximate search becomes worthwhile.

### 4.2 Embedding-Based Retrieval vs Keyword Search

| Aspect            | Keyword (BM25/TF-IDF) | Embedding-based        |
|-------------------|------------------------|------------------------|
| "decision"        | Finds "decision"       | Finds "let's go with", "agreed", "resolved" |
| Synonyms          | Misses them            | Captures them          |
| Typos             | Misses them            | Partially robust       |
| Exact terms       | Perfect                | Can miss exact matches |
| Speed (15K docs)  | <1ms                   | 2-10ms                 |
| Storage           | Inverted index, small  | 384 floats/doc, ~23MB  |

**Best practice:** Hybrid retrieval. Use keyword search for exact term
matching, embedding search for semantic similarity, and combine scores.
This is what production search systems (Vespa, Elasticsearch with
vectors) do.

### 4.3 Finding "Arcs" in Conversation

An arc is a thematic thread spanning multiple messages. Approaches:

1. **Sliding window similarity:** Embed each message. Compare each
   message's embedding to a running topic centroid. When similarity
   drops below a threshold, a new arc begins.
2. **Clustering:** Embed all messages, cluster with HDBSCAN or k-means.
   Each cluster is an arc. Works for retrospective analysis but not
   for streaming.
3. **Topic models:** BERTopic uses embeddings + HDBSCAN + c-TF-IDF to
   extract topics. Produces human-readable topic labels. Good for
   discovering what arcs exist.
4. **Reply chain / temporal proximity:** Combine embedding similarity
   with temporal distance. Messages close in time AND similar in
   embedding are likely part of the same arc.

---

## 5. Lightweight Local Models

### 5.1 ONNX Runtime

**What it is:** Cross-platform inference engine from Microsoft. Runs
neural network models exported to ONNX format. Available for
C/C++/Rust/Python/JS.

- **Rust crate:** `ort` (formerly `onnxruntime-rs`). Well-maintained.
  Supports CPU and GPU inference.
- **Performance:** 2-5x faster than PyTorch for CPU inference. Enables
  running sentence transformers at ~3-5ms per sentence.
- **Model conversion:** Any Hugging Face model can be exported to ONNX
  with `optimum` library: `optimum-cli export onnx --model
  sentence-transformers/all-MiniLM-L6-v2 output_dir/`
- **Practical:** This is the most battle-tested path for running
  transformer models in Rust. The `ort` crate handles tokenization
  via the `tokenizers` crate (also from Hugging Face, written in
  Rust).

### 5.2 Quantized Models

**What it is:** Reduce model precision from float32 to int8 or int4.
Cuts model size by 2-4x and improves inference speed.

- **ONNX quantization:** `optimum` can produce int8-quantized ONNX
  models. all-MiniLM-L6-v2 goes from 80MB to ~22MB.
- **GGML/GGUF quantization:** Popular for LLMs, less common for
  embedding models. Some embedding models are available in GGUF
  format.
- **Accuracy trade-off:** int8 quantization typically loses <1%
  accuracy on embedding quality benchmarks. int4 loses 2-5%.

### 5.3 Candle (Rust ML Framework)

**What it is:** Hugging Face's pure-Rust ML framework. No C++
dependencies (optional CUDA/Metal backends). Can load and run
transformer models.

- **Strengths:** Pure Rust. No Python. No ONNX conversion needed (can
  load safetensors directly). Supports Metal acceleration on Apple
  Silicon.
- **Weaknesses:** Smaller ecosystem than ONNX Runtime. Fewer
  pre-optimized model implementations. Some models need manual porting.
  Performance is good but not always on par with ONNX Runtime.
- **Supported models:** BERT, sentence transformers, LLMs (Llama,
  Mistral, Phi). The `candle-transformers` crate has implementations
  for common architectures.
- **Practical for bpmux:** Viable. The `candle-nn` and
  `candle-transformers` crates can load all-MiniLM-L6-v2. Embedding
  inference works. The main friction is that fewer people have trodden
  this path compared to ONNX, so documentation and examples are
  thinner.

### 5.4 What's Practical for a CLI Tool Without GPU

**Tier 1 (no model, instant startup):**
- TF-IDF + Naive Bayes/SVM: No model file. < 1MB memory. < 1ms
  inference. Build from word counts at startup.
- Keyword/regex rules: Zero overhead.

**Tier 2 (small model, fast startup):**
- Quantized all-MiniLM-L6-v2 via ONNX: ~22MB model file. ~50ms model
  load. ~5ms per embedding. Good accuracy.
- Binary size increase: ~10-15MB for ONNX Runtime dependency.

**Tier 3 (larger model, slower startup):**
- all-mpnet-base-v2 or DeBERTa via ONNX: 100-400MB model file.
  ~200ms model load. ~20ms per embedding. Best accuracy.
- Impractical for a CLI tool that starts fresh each invocation.
  Fine if the model stays loaded (daemon/server mode).

**Recommendation for a CLI tool:** Tier 1 for the default experience.
Tier 2 as an opt-in "enhanced" mode that downloads the model on first
use. Tier 3 only if running as a long-lived process.

---

## 6. Hybrid Approaches

Real-world systems almost never use a single technique. The best
results come from combining approaches at different stages.

### 6.1 Keyword Matching + Embeddings

**Pattern:** Use keyword/regex rules as a fast first pass, then
embeddings for messages that don't match any rule.

```
Message arrives
  -> Check regex rules (e.g., starts with "DECISION:", contains "s/old/new/")
  -> If match: classify immediately (<0.1ms)
  -> If no match: embed and classify with ML model (~5ms)
```

**Why it works:** Many categories have strong lexical signals.
Corrections often contain "s/old/new/" or "I meant" or "typo".
Decisions often contain "DECISION" or "agreed" or "let's go with".
Capture the easy cases with rules; spend compute on the ambiguous ones.

### 6.2 Rule-Based Pre-Filtering + ML Classification

**Pattern:** Use structural features to narrow the category space
before applying a classifier.

```
Message arrives
  -> Extract structural features:
     - Length (short messages are rarely decisions)
     - Contains question mark (likely question or discussion)
     - Contains code block (likely technical discussion)
     - Is a reply to another message (context-dependent)
     - Author role (PM vs developer vs designer)
  -> Use features to select candidate categories
  -> Apply ML classifier only among candidate categories
```

### 6.3 Cascade Architecture

**Pattern:** Fast, cheap classifier first; expensive, accurate
classifier only when confidence is low.

```
Message arrives
  -> Stage 1: TF-IDF + Logistic Regression (< 1ms)
  -> If confidence > 0.85: use this classification
  -> Stage 2: Embedding + k-NN (~5ms)
  -> If confidence > 0.80: use this classification
  -> Stage 3: Flag for human review
```

**Why it works:** Most messages (60-80%) are easy to classify. Only
the ambiguous 20-40% need the expensive model. Average inference time
drops significantly.

### 6.4 When to Use Which Approach

- **Keyword matching alone:** When categories are defined by explicit
  markers (DECISION:, ACTION:, etc.). If the chat has conventions,
  exploit them.
- **ML alone:** When categories are semantic and lack lexical markers.
  "I think we should go with the simpler design" is a decision but
  has no keywords.
- **Hybrid:** Almost always better than either alone. Start with rules,
  add ML for what rules miss.

---

## 7. Rust Ecosystem

### 7.1 Available Crates

**ML / Classification:**
- **`linfa`** (v0.7+): Rust's scikit-learn equivalent. Includes
  `linfa-bayes` (Naive Bayes), `linfa-svm` (SVM), `linfa-logistic`
  (Logistic Regression), `linfa-trees` (Decision Trees),
  `linfa-clustering` (k-means, DBSCAN). Mature enough for production
  on traditional ML tasks.
- **`smartcore`**: Alternative to linfa. Naive Bayes, SVM, Random
  Forest, etc. Less actively maintained than linfa but functional.

**Neural Networks / Transformers:**
- **`candle`** + **`candle-transformers`** + **`candle-nn`**: Hugging
  Face's Rust ML framework. Can run BERT/sentence transformer
  inference. Pure Rust.
- **`ort`** (ONNX Runtime bindings): Most reliable path for running
  neural network models in Rust. Well-maintained.
- **`tokenizers`**: Hugging Face's tokenizer library, written in Rust.
  Handles BPE, WordPiece, etc. Required for any transformer model.

**Text Processing:**
- **`rust-stemmers`**: Snowball stemming.
- **`stop-words`**: Stop word lists.
- **`unicode-segmentation`**: Proper word/sentence segmentation.
- **`whatlang`**: Language detection.
- **`regex`**: Rust's regex crate. Very fast.

**Vector Search:**
- **`usearch`**: Rust bindings for USearch (HNSW vector index). Small,
  fast, no server.
- **`hora`**: Pure Rust approximate nearest neighbor library. HNSW and
  other algorithms.
- **`qdrant-client`**: Client for Qdrant vector database.
- **`faiss`**: Rust bindings exist but are less mature.

**Serialization / Storage:**
- **`serde`** + **`bincode`**: Fast binary serialization for models
  and embeddings.
- **`memmap2`**: Memory-mapped files for large embedding stores.

### 7.2 Can We Do Useful Classification in Pure Rust Without Python?

**Yes, but with caveats.**

**What works well in pure Rust today:**
- TF-IDF computation (manual, ~100 lines of code)
- Naive Bayes, SVM, Logistic Regression (via linfa)
- Tokenization (via tokenizers crate)
- ONNX model inference (via ort)
- Sentence transformer inference (via candle or ort)
- Vector similarity search (via usearch or hora)
- Keyword/regex matching (via regex)

**What still needs Python (or workarounds):**
- SetFit fine-tuning (Python-only library)
- BERTopic (Python-only)
- Model conversion to ONNX (one-time, use optimum CLI)
- Hyperparameter tuning (easier in Python with scikit-learn)

**Practical approach:** Use Python for training/fine-tuning/exploration.
Export models to ONNX or safetensors. Run inference in Rust. This is
what the industry does; even Hugging Face's own production services
run Rust inference.

### 7.3 Recommended Crate Stack for bpmux

```
Tier 1 (rules + traditional ML):
  regex          - keyword/pattern matching
  tokenizers     - word tokenization
  linfa-bayes    - Naive Bayes classifier
  linfa-svm      - SVM classifier (if needed)
  serde/bincode  - model serialization

Tier 2 (add embeddings):
  ort            - ONNX Runtime for sentence transformer inference
  tokenizers     - tokenize input for the model
  usearch/hora   - vector similarity search
  memmap2        - memory-mapped embedding storage

Tier 3 (pure Rust, no ONNX dependency):
  candle         - native Rust transformer inference
  candle-nn      - neural network layers
  candle-transformers - pre-built model architectures
  tokenizers     - tokenization
```

---

## 8. Recommended Progression

A practical path from simplest to most sophisticated, each stage
building on the last.

### Stage 1: Keyword Rules (Day 1)

- Regex patterns for known markers: "DECISION:", "s/.../", "ACTION:",
  "?", etc.
- Structural heuristics: message length, contains code block, is
  question.
- **Effort:** A few hours. **Accuracy:** 40-60% (catches the obvious
  cases). **Dependencies:** `regex` crate only.
- **Value:** Immediate. Classifies the easy messages. Reveals which
  categories are hard.

### Stage 2: TF-IDF + Naive Bayes (Week 1)

- Hand-label 200-500 messages (use Stage 1 output as suggestions to
  speed up labeling).
- Train Naive Bayes on TF-IDF features.
- Combine with keyword rules (rules override when confident).
- **Effort:** 1-2 days for labeling, half day for implementation.
  **Accuracy:** 70-82%. **Dependencies:** `linfa-bayes`, `tokenizers`.
- **Value:** Covers the messages that keyword rules miss. Provides a
  baseline for measuring improvements.

### Stage 3: Embeddings + k-NN or Logistic Regression (Week 2-3)

- Add a sentence transformer model (all-MiniLM-L6-v2, quantized ONNX).
- Embed all labeled messages. Train logistic regression on embeddings
  or use k-NN.
- Use as the backend for messages where Stage 1-2 are low confidence.
- **Effort:** 2-3 days. **Accuracy:** 82-90%. **Dependencies:** `ort`,
  `tokenizers`, model file (~22MB).
- **Value:** Catches semantic similarity that TF-IDF misses.

### Stage 4: SetFit Fine-Tuning (Week 3-4)

- Use the labeled data from Stage 2 to fine-tune a sentence
  transformer with SetFit (Python, one-time).
- Export to ONNX, replace the generic sentence transformer.
- **Effort:** 1 day. **Accuracy:** 85-92%. **Dependencies:** Same as
  Stage 3 (different model weights).
- **Value:** The fine-tuned model's embedding space is shaped for your
  specific categories, improving separation.

### Stage 5: Arc Detection with Semantic Search (Ongoing)

- Store all message embeddings in a vector index.
- Implement sliding-window topic detection.
- Enable queries like "find all messages about the auth redesign".
- **Effort:** 3-5 days. **Dependencies:** `usearch` or `hora`,
  `memmap2`.
- **Value:** Unlocks conversation-level intelligence beyond per-message
  classification.

### What to Skip

- **Zero-shot NLI:** Too slow and too large for CLI use. Good for
  initial exploration in a Jupyter notebook but not for production.
- **Full LLM classification:** Defeats the "no API calls" constraint.
  Local LLMs (7B+) are too large for a CLI tool.
- **Deep learning from scratch:** Not enough data (15K messages) to
  train a neural classifier from scratch. Use pre-trained models.

---

## 9. Key Recommendations for bpmux

1. **Start with rules.** Chat messages often have structural signals
   that are free to exploit. Rules are fast, interpretable, and
   debuggable. They also produce labeled data for the ML stages.

2. **Invest in labeling, not algorithms.** The difference between 200
   and 1000 labeled examples matters more than the difference between
   Naive Bayes and BERT for most classification tasks.

3. **Use ONNX Runtime (`ort`) for neural models in Rust.** It is more
   mature and better documented than Candle for inference. Candle is
   promising but ONNX is the safer bet today.

4. **Keep the model optional.** The CLI should work without downloading
   any model (keyword rules + TF-IDF). Enhanced classification with
   embeddings should be opt-in and download the model on first use.

5. **Hybrid always wins.** No single technique covers all cases. The
   cascade of rules -> traditional ML -> embeddings gives the best
   accuracy-per-millisecond ratio.

6. **Embeddings are the foundation for arc detection.** Once you have
   embeddings, classification and semantic search share the same
   infrastructure. Build the embedding pipeline once and use it for
   both.

7. **Python for training, Rust for inference.** Do not fight the
   ecosystem. Use Python notebooks for exploration, labeling tools,
   and fine-tuning. Export to ONNX. Ship Rust.
