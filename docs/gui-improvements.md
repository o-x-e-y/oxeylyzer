# GUI Improvement Pass — Findings & Plan

Branch: `feature/gui-improvements`. Scope per user: settings, features, UX, bugs.
Visual design is settled — no redesign. Competitive context: Cyanophage playground,
patorjk KLA / KLAnext, keysolve, genkey, trialyzer, and oxeylyzer-2 (o2.oxey.dev).

## Bugs found (code-confirmed)

| # | Severity | Bug | Root cause |
|---|---|---|---|
| B1 | High | **Cancel during generation doesn't cancel.** | `start_generate` drives `generate_n_with_pins_iter(count, ...).for_each(...)` — rayon runs all `count` items regardless of `cancel_flag`; the flag only stops result *collection*. Frontend additionally resets `running` immediately, allowing a second overlapping run that races the first for `state.generated`. |
| B2 | High | **Saving one generated layout breaks the rest / view state randomly resets.** | Any write into the layouts dir (e.g. `save_generated`) triggers the file watcher → `reload_state` clears `state.generated` (backend) and emits `config-reloaded` → `initStore()` sets `loading=true` → all views unmount, losing local state (generation results, listeners, compare picks). Saving result #1 then #2 → "Index out of bounds". |
| B3 | Medium | **Bigram lists go stale after swaps/disables in Analyze.** | `getBigrams(baseName, ...)` always queries the saved layout by name; there is no backend command for a custom arrangement, so SFB/scissor/… lists silently describe the *original* layout while keyboard + stats show the modified one. |
| B4 | Medium | **`set_config` leaves stale generated results.** | Unlike `set_language`, it doesn't clear `state.generated`; saved results then use the old engine's mapping (potential garbage keys on save). |
| B5 | Low | Renaming in Edit forks but leaves the old file + name/file mismatch when overwriting with a changed name. No delete exists to clean up. |
| B7 | Low | `lookup_ngram` with characters not in the corpus silently maps to byte 0 (REPLACEMENT) and reports its frequency instead of an error. |
| B9 | Medium | `load_corpus` is a synchronous command — the UI freezes for the whole corpus processing with no progress. |
| B10 | Low | `KeyboardDisplay` mounts drag-and-drop machinery even when `draggable` is unset — keys can be picked up (and snap back) in read-only contexts. |

## Feature gaps vs the ecosystem ("make it whole")

| # | Feature | Notes |
|---|---|---|
| F1 | **Per-finger usage + finger speed bars, hand balance** | Every major analyzer (Cyanophage, KLA, genkey) shows this. `finger_speed[10]` is already in the DTO but *never rendered*; finger usage % needs a small DTO addition. |
| F2 | **Trigram drill-down lists** | We list bigrams only. Add per-category top trigrams (rolls, redirects, alternates, onehands, sfts) as tabs next to the bigram tabs, with key highlighting on hover. |
| F3 | **Search algorithm selector in Generate** | The Engine trait + tuned ILS/SA/LAHC from `feature/search-engines` (merged into this branch's history) make this a dropdown + match. ILS at (5,25) reliably finds the optimum — a real quality upgrade over the plain hill climb. |
| F4 | **Delete layout** | Missing entirely (REPL has `remove`). Backend command + confirm UI in Layouts view; also fixes the rename-leftover problem (B5). |
| F5 | **LayoutSearch shows list on focus** | The user's own `tauri-changes.md` note: browsing before typing. Show ranked list (or all, alphabetical) when focused with empty query. |
| F6 | **Keep partial results on cancel** | After a real cancel (B1), show what was generated so far instead of discarding. |
| F7 | **Save-as from modified Analyze state** | After drag-swapping in Analyze there's no way to persist the arrangement; add "Save as…" that writes a .dof derived from the base layout. |
| F8 | **Heatmap legend + persisted color scheme** | Small legend strip; persist `heatScheme` in the session file. |

Deferred (bigger or out of scope for this pass): paste-text quick analysis, n-way compare,
magic/thumb keys (core limitation), from-scratch layout creation, URL-style sharing.

## Implementation order

1. **Backend bugs**: B1 (batched cancellable generation + emit partial results→F6), B2 (stop clearing `generated` on watcher reload; only clear on language/config change → B4; frontend: refresh store without unmount), B3 (new `get_bigrams_custom`), B7, B9 (async corpus load).
2. **Backend features**: finger usage in DTO (F1), trigram lists command (F2), algorithm param on `start_generate` (F3), `delete_layout` (F4).
3. **Frontend**: Generate view (algorithm dropdown, cancel/partial UX), Analyze (custom bigrams, finger bars, trigram tabs, save-as), Layouts (delete), LayoutSearch focus list (F5), heatmap legend + session-persisted scheme (F8), non-unmounting store refresh, B10 conditional dnd.
4. Verify: `cargo check`/`clippy` on src-tauri, `bun run build`, `bun eslint src` in oxeylyzer-tauri; manual TUI-equivalent run is left to the user (needs a display).
