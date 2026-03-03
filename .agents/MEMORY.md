# Project Memory

## 2026-03-03 — Hybrid Storage Architecture
**Fact/Decision:** Chose a three-tier storage strategy: Markdown for durability, libSQL for structured relations, and LanceDB for semantic vector recall.
**Why it matters:** Balances human readability with high-performance AI retrieval and structured knowledge mapping.

## 2026-03-03 — Dependency Trimming
**Fact/Decision:** Removed async playground dependencies (flume, async-task, reqwest) and kept only core KB requirements.
**Why it matters:** Keeps the binary small, focused, and reduces maintenance overhead for the knowledge base.
