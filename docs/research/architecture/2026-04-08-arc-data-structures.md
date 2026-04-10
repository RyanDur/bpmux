# Arc Data Structures: Research

**Date:** 2026-04-08
**Context:** Selecting data structures for "arcs" — reasoning paths
extracted from bpmux session logs. Arcs are message sequences that
capture how understanding formed. The session log is append-only JSONL,
growing ~11k lines/week.

**Requirements recap:**
- Many-to-many arc-lesson relationships
- Nestable/composable arcs (sub-arcs)
- Weighted (frequency, recency, source authority)
- Searchable by topic, sender, time range
- Efficient at scale (months of conversation, 500k+ lines)

---

## 1. Tree Structures

### B-Tree / B+ Tree

**Concept.** Self-balancing tree where nodes hold multiple keys and
have high fan-out. B+ trees store all values in leaf nodes (linked for
range scans); internal nodes are index-only.

**Big O.**
| Operation    | Complexity       |
|-------------|------------------|
| Insert       | O(log n)         |
| Point lookup | O(log n)         |
| Range query  | O(log n + k)     |
| Update       | O(log n)         |

Where n = number of indexed entries, k = number of results in range.

**Applicability.** B+ trees are the standard for time-based indexing.
Index messages by timestamp in a B+ tree and range queries ("all
messages between T1 and T2") become a leaf-level scan after a single
tree descent. Also suitable for indexing by sender (sorted by sender
string, then timestamp within sender). This is the same structure
backing SQLite, RocksDB, and sled under the hood.

**Rust crates.**
- `sled` — embedded B+ tree database, pure Rust, lock-free. Production
  use is common but the author has paused active development. Still
  works well for read-heavy embedded use.
- `redb` — newer embedded B+ tree KV store, single-writer MVCC, active
  development. Becoming the preferred sled alternative.
- `rocksdb` (via `rust-rocksdb`) — FFI wrapper around RocksDB. Battle-tested
  at massive scale but brings C++ dependency.

**Verdict: Strong fit for primary time and sender indexes.**

---

### Trie / Radix Tree (Patricia Trie)

**Concept.** Tree keyed by string prefixes. A radix tree compresses
single-child chains into one edge, saving space. Lookup follows the
key character by character.

**Big O.**
| Operation    | Complexity       |
|-------------|------------------|
| Insert       | O(m)             |
| Lookup       | O(m)             |
| Prefix query | O(m + k)         |

Where m = key length, k = results matching prefix.

**Applicability.** Useful for topic-prefix searches ("design/" matches
"design/arc-model", "design/ui", etc.) if topics are hierarchical
strings. Also good for auto-complete on topic names. Less useful for
time-range queries.

**Rust crates.**
- `radix_trie` — straightforward radix trie.
- `qp-trie` — QP-trie (quantized popcount), faster for ASCII keys.
- `cedarwood` — double-array trie, very fast for dictionary-style lookups.

**Verdict: Good for topic hierarchy lookups. Secondary structure, not
primary.**

---

### Segment Tree

**Concept.** Binary tree where each node represents an interval
(segment) of an array. Supports aggregate queries over ranges (sum,
min, max, count) in logarithmic time.

**Big O.**
| Operation       | Complexity   |
|----------------|--------------|
| Build           | O(n)         |
| Range query     | O(log n)     |
| Point update    | O(log n)     |

**Applicability.** If you need aggregates like "how many arcs touched
topic X between day 5 and day 30" or "what's the maximum weight of any
arc in this time range," segment trees answer that fast. Less useful for
retrieval of actual arc data; more for analytics queries.

**Rust crates.**
- `segment-tree` — basic implementation.
- Usually hand-rolled since the structure is simple and the query
  function is domain-specific.

**Verdict: Niche. Useful for weight-based analytics queries over time
ranges. Not a primary structure.**

---

## 2. Graph Structures

### DAG (Directed Acyclic Graph)

**Concept.** Nodes connected by directed edges, no cycles. Natural
representation for arc-lesson relationships: arcs point to lessons,
lessons point back to arcs, sub-arcs point to parent arcs. The acyclic
constraint fits because reasoning flows forward in time.

**Big O (adjacency list).**
| Operation          | Complexity   |
|-------------------|--------------|
| Add node           | O(1)         |
| Add edge           | O(1)         |
| Find neighbors     | O(degree)    |
| Path query (BFS)   | O(V + E)     |
| Topological sort   | O(V + E)     |

**Applicability.** The arc-lesson many-to-many relationship is a
bipartite subgraph within the DAG. Modeling:
- Arc nodes, Lesson nodes
- Arc -> Lesson edges (this arc teaches this lesson)
- Lesson -> Arc edges (this lesson is evidenced by this arc)
- Arc -> Sub-Arc edges (containment/composition)

Traversal answers questions like "what arcs led to this lesson?" or
"what lessons did this arc sequence produce?" Weighted edges carry
frequency, recency, authority scores.

**Storage model.** Adjacency lists stored in a flat structure:
```
arcs:      HashMap<ArcId, Arc>
lessons:   HashMap<LessonId, Lesson>
arc_edges: HashMap<ArcId, Vec<(LessonId, Weight)>>
sub_arcs:  HashMap<ArcId, Vec<ArcId>>
```

This is O(1) for edge insertion, O(degree) for neighbor lookup.

**Rust crates.**
- `petgraph` — the standard Rust graph library. Supports directed
  graphs, DAG operations, topological sort, all traversals. Mature,
  well-tested, widely used. This is the default choice.
- `daggy` — DAG-specific wrapper around petgraph. Enforces acyclicity
  at the type level.

**Verdict: Core structure for arc-lesson relationships. Use petgraph
or a custom adjacency-list model.**

---

### Knowledge Graph / Property Graph

**Concept.** Nodes and edges both carry typed properties. Enables
queries like "find all arcs where sender=designer AND topic contains
'layout' AND weight > 0.7."

**Applicability.** A property graph is the conceptual model for the
full arc system. Practically, you don't need a graph database — you
need the graph model implemented over your storage layer. The
properties (topic, sender, weight, time range) live on the arc/lesson
structs; the graph edges live in adjacency lists.

If the system grows to need SPARQL-like queries across many entity
types, consider:

**Rust crates.**
- `oxigraph` — full RDF/SPARQL engine in Rust. Overkill for the
  current scope but worth knowing about.
- `cozo` — embedded Datalog database, good for recursive graph queries.
  Novel, actively developed, Rust-native.

**Verdict: The conceptual model is a property graph. Implementation
should be custom structs + petgraph, not a graph database (yet).**

---

## 3. Inverted Index

**Concept.** Maps terms to the set of documents containing them.
Given a corpus of messages, build a mapping:
```
"refactor"  -> [msg_42, msg_108, msg_305]
"websocket" -> [msg_7, msg_42, msg_200]
```
Searching for "refactor AND websocket" intersects the two postings
lists: [msg_42].

**Big O.**
| Operation            | Complexity           |
|---------------------|----------------------|
| Index one document   | O(terms in doc)      |
| Boolean search       | O(min postings list) |
| Ranked search (TF-IDF/BM25) | O(postings * log n) |

**How search engines do it.** Lucene (Java) and tantivy (Rust) both:
1. Tokenize text, apply stemming/lowercasing
2. Build postings lists per term, stored as compressed integer arrays
   (delta-encoded, SIMD-decoded)
3. Store positional data for phrase queries
4. Support real-time indexing via in-memory segments merged to disk

**Applicability.** This is how you search for arcs by topic when
topics are free-text (not just tags). Index each message's content.
Then an arc search becomes: find messages matching the query, then
find which arcs contain those messages.

For bpmux, tantivy is the obvious choice — it's a Rust-native search
engine library modeled after Lucene.

**Rust crates.**
- `tantivy` — full-text search engine library. Supports BM25 ranking,
  phrase queries, faceted search, real-time indexing. Battle-tested (powers
  Quickwit, a distributed search engine). This is the gold standard for
  Rust text search.
- `meilisearch` — if you want a search server rather than a library
  (HTTP API, typo tolerance). Also Rust.

**Verdict: Use tantivy for topic/content search over messages. Primary
search structure.**

---

## 4. Skip Lists and LSM Trees

### Skip List

**Concept.** Linked list with multiple levels of express lanes.
Probabilistic alternative to balanced trees. Each element is promoted
to higher levels with probability p (usually 1/2).

**Big O.**
| Operation    | Expected    | Worst case |
|-------------|-------------|------------|
| Insert       | O(log n)    | O(n)       |
| Search       | O(log n)    | O(n)       |
| Delete       | O(log n)    | O(n)       |

**Applicability.** Skip lists are the in-memory component of many
LSM trees (the "memtable"). They're good for ordered data with
concurrent access — multiple readers, single writer. If you're building
a custom in-memory index over recent messages before flushing to disk,
a skip list is a good memtable.

**Rust crates.**
- `crossbeam-skiplist` — concurrent skip list from the crossbeam
  project. Lock-free, production quality.

---

### LSM Tree (Log-Structured Merge Tree)

**Concept.** Write-optimized structure for append-heavy workloads.
Writes go to an in-memory buffer (memtable, often a skip list). When
full, the memtable is flushed to disk as a sorted run (SSTable).
Background compaction merges SSTables. Reads check memtable first,
then SSTables (newest to oldest), using bloom filters to skip
irrelevant SSTables.

**Big O.**
| Operation    | Complexity                |
|-------------|---------------------------|
| Write        | O(1) amortized            |
| Point read   | O(log n) * L levels       |
| Range read   | O(log n + k) * merge cost |
| Compaction   | O(n) per level            |

**Applicability.** The session log is append-only JSONL. An LSM tree
is the natural indexing strategy: buffer incoming messages in memory,
periodically flush sorted indexes to disk. This is exactly what
RocksDB and sled do internally. You get this for free by using either.

The key insight: don't build a raw LSM tree. Use an embedded database
(redb, sled, RocksDB) that implements one.

**Rust crates.**
- `redb` — uses a B+ tree internally but has LSM-like write
  performance characteristics. Simple API, good for embedded use.
- `sled` — LSM-inspired (uses a Bw-tree, which is log-structured).
- `rust-rocksdb` — actual LSM tree (RocksDB). If you need write
  throughput at the expense of C++ dependency.
- `agatedb` — Rust port of BadgerDB (LSM-based). Less mature.

**Verdict: Don't build an LSM tree — use an embedded DB that
implements one. redb or sled for pure Rust; RocksDB if you need
proven scale.**

---

## 5. Interval Trees

**Concept.** BST augmented so each node stores an interval [lo, hi]
and the subtree's maximum hi value. Efficiently finds all intervals
overlapping a query point or interval.

**Big O.**
| Operation              | Complexity       |
|-----------------------|------------------|
| Insert                 | O(log n)         |
| Find all overlapping   | O(log n + k)     |
| Delete                 | O(log n)         |

Where k = number of overlapping intervals.

**Applicability.** Each arc spans a time interval [start_time,
end_time]. Queries like "find all arcs active during this meeting" or
"what arcs overlap with this time window" are interval overlap queries.
An interval tree answers these in O(log n + k).

Without an interval tree, you'd scan all arcs and check overlap — O(n).
At 500k+ messages producing thousands of arcs, the difference matters.

Also applicable to nested arcs: if arc A contains sub-arc B, then
B's interval is contained within A's. An interval tree naturally
represents this containment.

**Rust crates.**
- `bio::data_structures::interval_tree` (from the `bio` bioinformatics
  crate) — well-tested interval tree. Works for any Ord type.
- `rust-lapper` — interval overlap library, optimized for genomics
  but general-purpose. Very fast for batch queries.
- `intervaltree` — simple standalone crate.
- `coitrees` — cache-oblivious interval tree, extremely fast for
  static datasets (build once, query many). Ideal if arcs are
  mostly immutable once identified.

**Verdict: Strong fit for time-range queries on arcs. Use coitrees
if arcs are mostly static, rust-lapper if they change.**

---

## 6. Bloom Filters

**Concept.** Probabilistic set membership test. A bit array with k
hash functions. Insert: set k bits. Query: check if all k bits are
set. False positives possible; false negatives impossible.

**Big O.**
| Operation | Complexity |
|-----------|-----------|
| Insert     | O(k)      |
| Query      | O(k)      |
| Space      | ~10 bits per element for 1% FP rate |

k is typically 3-7, so effectively O(1).

**Applicability.** Use cases:
1. "Does topic X have any arcs?" — check a bloom filter before
   hitting the inverted index. If the filter says no, skip the
   search entirely. Saves disk I/O.
2. "Has this message already been assigned to an arc?" — dedup check
   during arc detection.
3. Per-SSTable bloom filters (if using LSM storage) to skip tables
   during reads. RocksDB and sled do this automatically.

At 10k topics, a bloom filter with 1% false positive rate costs ~12 KB.
Trivial memory cost for significant I/O savings.

**Counting bloom filters** allow deletion (each slot is a counter, not
a bit). Useful if topics can be removed from arcs.

**Cuckoo filters** offer similar functionality with better space
efficiency and support for deletion. Prefer over counting bloom
filters.

**Rust crates.**
- `bloomfilter` — simple, correct bloom filter.
- `probabilistic-collections` — bloom, cuckoo, count-min sketch, HLL.
- `xorf` — xor filters (successor to bloom/cuckoo, better space
  efficiency, but static — no insertion after build).
- `scalable_bloom_filter` — grows as elements are added.

**Verdict: Cheap optimization layer. Use xor filters for static topic
sets, bloom filters for dynamic ones. Tiny memory footprint, big I/O
savings.**

---

## 7. Big O Summary

All operations relevant to the arc system:

| Structure       | Insert (new msg) | Search (topic) | Range query (time) | Update (weight) |
|----------------|------------------|----------------|--------------------|-----------------|
| B+ tree         | O(log n)         | O(log n)       | O(log n + k)       | O(log n)        |
| Trie            | O(m)             | O(m)           | N/A                | O(m)            |
| Segment tree    | O(log n)         | N/A            | O(log n)           | O(log n)        |
| DAG (adj list)  | O(1)             | O(degree)      | N/A                | O(1)            |
| Inverted index  | O(terms)         | O(postings)    | N/A                | N/A             |
| Skip list       | O(log n)         | O(log n)       | O(log n + k)       | O(log n)        |
| LSM tree        | O(1) amortized   | O(log n * L)   | O(log n + k)       | O(1) amortized  |
| Interval tree   | O(log n)         | N/A            | O(log n + k)       | O(log n)        |
| Bloom filter    | O(1)             | O(1) (approx)  | N/A                | N/A             |

**Key:** n = total entries, k = results in range, m = key length,
L = LSM levels, terms = words in document, postings = documents
matching term, degree = edges from node.

---

## 8. Recommended Rust Crates (Battle-Tested)

| Need                      | Crate              | Maturity   | Notes                          |
|--------------------------|--------------------|------------|--------------------------------|
| Embedded KV / B+ tree    | `redb`             | Active     | Pure Rust, ACID, single-writer |
| Embedded KV (alternative) | `sled`            | Stable     | Lock-free, log-structured      |
| Full-text search          | `tantivy`          | Mature     | Powers Quickwit. Gold standard |
| Graph (DAG)               | `petgraph`         | Mature     | De facto Rust graph library    |
| Interval queries          | `coitrees`         | Stable     | Fastest for static intervals   |
| Interval queries (mutable)| `rust-lapper`      | Stable     | Good for dynamic intervals     |
| Bloom / probabilistic     | `xorf`             | Stable     | Xor filters, very compact      |
| Bloom (dynamic)           | `bloomfilter`      | Stable     | Simple, correct                |
| Concurrent skip list      | `crossbeam-skiplist`| Mature    | From crossbeam project         |
| Radix trie                | `radix_trie`       | Stable     | For topic prefix lookups       |
| Serialization             | `bincode` or `rkyv`| Mature     | Fast binary serde              |

---

## 9. Composite Architecture Sketch

How these pieces fit together for the arc system:

```
Session Log (JSONL, append-only)
        |
        v
  [Message Ingestion]
        |
        +---> redb (B+ tree index: timestamp, sender, arc_id)
        |
        +---> tantivy (inverted index: message content, topic tags)
        |
        +---> bloom filter (topic existence check)
        |
        v
  [Arc Detection / Extraction]
        |
        v
  petgraph DAG (arc <-> lesson relationships, sub-arc nesting)
        |
        +---> coitrees (interval tree: arc time ranges)
        |
        +---> weight store in redb (frequency, recency, authority)
```

**Query paths:**
- "Find arcs about topic X" -> bloom filter (fast reject) -> tantivy
  (find messages) -> redb (find arc_ids for those messages) -> petgraph
  (traverse to lessons)
- "Find arcs in time range" -> coitrees (interval overlap) -> redb
  (load arc data)
- "What lessons came from arcs involving designer?" -> redb (sender
  index -> messages -> arc_ids) -> petgraph (arc -> lesson edges)
- "Sub-arcs of arc A" -> petgraph (children in DAG)
- "Update arc weight" -> redb (point update), O(log n)

---

## 10. Key Takeaways

1. **Don't build storage from scratch.** Use `redb` (or `sled`) as
   the embedded database. It handles B+ trees, crash recovery, and
   concurrent reads. The append-only JSONL log remains the source of
   truth; the DB is a derived index.

2. **tantivy for search.** Full-text search is a solved problem in
   Rust. tantivy gives you BM25 ranking, phrase queries, and real-time
   indexing out of the box.

3. **petgraph for relationships.** The arc-lesson many-to-many and
   arc-subarc nesting are graph problems. petgraph is the right tool.

4. **Interval trees for time ranges.** coitrees is remarkably fast
   for "find all arcs overlapping this window." Worth the small
   dependency.

5. **Bloom/xor filters are free wins.** 12 KB of memory saves
   disk I/O on every topic query. Add them early.

6. **The JSONL log stays.** All indexes are derived and rebuildable.
   If the index format changes, replay the log. This is the LSM
   philosophy applied at the application level.
