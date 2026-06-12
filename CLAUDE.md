# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Run

```bash
# Run the REPL (debug)
cargo run

# Run the REPL (optimized)
cargo run --release

# Run all tests
cargo test

# Run tests for a specific crate
cargo test -p oxeylyzer-core

# Run a specific test
cargo test -p oxeylyzer-core -- generate

# Run benchmarks (uses diol, not criterion)
cargo bench --bench generation

# Run a specific bench by name
cargo bench --bench generation -- score_swap

# Run the search-algorithm comparison suite (ratatui TUI; q/Esc quits)
cargo run --release -p oxeylyzer-bench -- -n 100

# Headless (markdown table): -a picks algorithms, -t sets a per-algorithm time budget in seconds
cargo run --release -p oxeylyzer-bench -- -n 50 -a hill,sa,ils --headless
```

The toolchain is `stable` Rust (see `rust-toolchain.toml`). If a tool is missing, use `nix-shell -p <package> --run "<command>"`.

## Workspace Layout

Four crates:

- **`oxeylyzer-core`** — the library. All analysis and generation logic lives here.
- **`oxeylyzer-repl`** — the interactive REPL shell built on `xflags` + `sexp` parsing.
- **`oxeylyzer-resources`** — resolves platform-specific data directories (layouts, corpora, etc.) and handles downloading/seeding data.
- **`oxeylyzer-bench`** — terminal suite (ratatui) that benches the `Engine` search algorithms against each other (best score, top-5 mean, duplicate %, layouts/sec; fixed-count `-n` or fixed-time `-t` budgets).
- **`oxeylyzer-tauri/src-tauri`** — Tauri v2 desktop GUI frontend. Develop with `bun run tauri dev` inside `oxeylyzer-tauri/`.

The root `Cargo.toml` is both the workspace manifest and the binary crate; the binary entry point is `oxeylyzer-repl/src/bin/main.rs`.

## Core Architecture

### Data pipeline

```
Raw text files (static/text/<lang>/)
  → CorpusCleaner (corpus_cleaner.rs, rules from corpus_config/*.toml)
  → Data (data.rs) — character / bigram / trigram counts, serialized as JSON
  → AnalyzerData (analyzer_data.rs) — pre-weighted flat arrays (i64), baked from Data + Config
  → Oxeylyzer (generate.rs) — holds AnalyzerData + per-char trigram groups, entry point for scoring/generation
```

### Layout representation

- **`Layout`** (`layout.rs`) — human-readable DOF JSON format (libdof); used for I/O and storage.
- **`FastLayout`** (`fast_layout.rs`) — internal representation. Keys are `u8` indices (via `CharMapping`), not chars. Precomputes `FSpeedIndices`, `ScissorIndices`, `LsbIndices`, `PinkyRingIndices`, `StretchIndices`, `UsageIndices`, and `possible_swaps` at construction time so scoring loops never branch or filter.

Convert between them with `Oxeylyzer::fast_layout(&layout, pins)`.

### Scoring hot path

Scoring is integer-only (`i64`). Weights are scaled by 100× at startup and baked into flat arrays. The key types:

- `LayoutCache` — incremental score broken down by component (fspeed/usage per finger, trigrams, stretch, pinky-ring). A swap recalculates only the two affected fingers.
- `per_char_trigrams: HashMap<[u8; 2], TrigramData>` — trigrams grouped by character pair so a swap only re-evaluates the ~1% of trigrams that could have changed.
- `trigram_patterns: Arc<[TrigramPattern; 1000]>` — lookup table: `finger_a * 100 + finger_b * 10 + finger_c → TrigramPattern`. Zero branching.

### Generation algorithm

`Oxeylyzer::generate_n_iter(n, basis)` → Rayon parallel iterator. Each task: randomly shuffle unpinned keys → greedy hill-climb via `best_swap_cached` until no improving swap exists (up to 200 swaps). All shared data is `Arc`-wrapped; no locks needed.

### Search algorithms (`Engine` trait)

`oxeylyzer-core/src/generate/engine.rs` defines the `Engine` trait: one required method `generate_with_pins`, with default `generate` / `generate_n_iter` / `generate_n_with_pins_iter` (rayon) implementations. The RPIT iterator methods make it **not dyn-compatible** — `oxeylyzer-bench` dispatches via the `AlgorithmKind` enum instead. `Oxeylyzer` itself implements `Engine` (note: its inherent methods shadow the trait methods at call sites; use `Engine::generate(...)` UFCS to hit the trait).

Implementations in `oxeylyzer-core/src/generate/`, all structs holding `&Oxeylyzer` plus tunables:

- `hill_climber.rs` — `CachedHillClimber`, named wrapper over the built-in random-restart greedy climb (baseline).
- `annealing.rs` — `SimulatedAnnealing` (self-scaling initial temperature, geometric cooling, greedy polish).
- `ils.rs` — `IteratedLocalSearch` (climb → perturb k swaps → re-climb, keep best).
- `lahc.rs` — `LateAcceptanceHillClimbing` (ring buffer of past scores).
- `depth.rs` — `best_double_swap` (depth-2 search; can return a degenerate no-op pair at local optima, so callers must guard with strict `score > current`), `DepthTwoHillClimber` (full quadratic search per move), `VariableNeighborhoodDescent` (depth-2 only as escape at depth-1 optima; always ends on a climb).

The reusable greedy loop is `Oxeylyzer::climb(&mut layout, &mut cache, &swaps, max_swaps)` (`max_swaps == 0` means unlimited; cache must come from `initialize_cache`). Shared test fixtures: `generate/test_util.rs` (`analyzer()`, `qwerty()`, `score()`, `assert_valid()`).

Dead experiments (NOT declared as modules, do not compile): `generate/iterative.rs`, `desshaw.rs`, `michaelll.rs`. `obsolete.rs` holds the non-cached `best_swap`/`score_swap` used by tests to validate the cached path.

## Configuration

- `config.toml` (repo root) — analyzer weights and corpus path loaded at runtime.
- `languages_default.cfg` — 30-key character set per language used during generation.
- `corpus_config/<lang>.toml` — corpus cleaning rules per language (inherits, letter transforms, accent key mappings).
- Static data lives under `oxeylyzer-core/static/` (layouts as `.dof` JSON, precomputed language data as `.json`).

## Key Design Invariants

- All scoring is `i64`; `f64` weights are converted once at startup (`Weights → AnalyzerWeights`, `×100`).
- `Box<[T]>` is preferred over `Vec<T>` for fixed-size arrays (communicates immutability, no excess capacity).
- `Arc<[T]>` for data shared across Rayon threads.
- `CharMapping` translates `char ↔ u8`; every hot path works with `u8` indices only.
- `FastLayout::possible_swaps` excludes pinned positions; passing pins to `fast_layout()` bakes them in up front.
