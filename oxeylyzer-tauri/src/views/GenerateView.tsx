import { createMemo, createSignal, For, onCleanup, Show } from "solid-js";
import KeyboardDisplay from "../components/KeyboardDisplay";
import LayoutSearch from "../components/LayoutSearch";
import Dropdown from "../components/Dropdown";
import { appStore } from "../store";
import { startGenerate, saveGenerated, cancelGenerate } from "../api";
import type { Layout } from "../types";
import { listen } from "@tauri-apps/api/event";

type SaveState = { name: string; saved: boolean };

const ALGORITHMS: { id: string; label: string; hint: string }[] = [
  { id: "hill", label: "hill climb", hint: "fastest — many layouts per second" },
  { id: "ils", label: "iterated local search", hint: "best quality — finds the optimum most reliably" },
  { id: "sa", label: "simulated annealing", hint: "good quality, faster than ils" },
  { id: "lahc", label: "late acceptance", hint: "consistent results, slower" },
];

export default function GenerateView() {
  const [baseName, setBaseName] = createSignal(appStore.layouts[0]?.name ?? "");
  const [countStr, setCountStr] = createSignal("1000");
  const count = () => Math.max(1, parseInt(countStr()) || 1);
  const [visibleCount, setVisibleCount] = createSignal(10);
  const [pinnedChars, setPinnedChars] = createSignal<Set<string>>(new Set());
  const [algorithm, setAlgorithm] = createSignal("hill");

  const pins = createMemo(() => [...pinnedChars()].join(""));
  const baseLayout = createMemo(() => appStore.layouts.find((l) => l.name === baseName()));

  function togglePin(char: string) {
    setPinnedChars((prev) => {
      const next = new Set(prev);
      if (next.has(char)) next.delete(char);
      else next.add(char);
      return next;
    });
  }
  const [running, setRunning] = createSignal(false);
  const [cancelling, setCancelling] = createSignal(false);
  const [wasCancelled, setWasCancelled] = createSignal(false);
  const [progress, setProgress] = createSignal<{ done: number; total: number } | null>(null);
  const [results, setResults] = createSignal<Layout[]>([]);
  const [saveStates, setSaveStates] = createSignal<SaveState[]>([]);
  const [error, setError] = createSignal("");

  let unlisteners: (() => void)[] = [];

  const clearListeners = () => {
    unlisteners.forEach((u) => u());
    unlisteners = [];
  };

  onCleanup(clearListeners);

  function initSaveStates(layouts: Layout[]) {
    setSaveStates(layouts.map((l) => ({ name: l.name, saved: false })));
  }

  async function handleGenerate() {
    clearListeners();

    setRunning(true);
    setCancelling(false);
    setWasCancelled(false);
    setResults([]);
    setProgress({ done: 0, total: count() });
    setError("");

    const progressUnlisten = await listen<{ done: number; total: number }>(
      "generate-progress",
      (e) => setProgress(e.payload),
    );

    // A single done event carries the results; `cancelled: true` means the run
    // was stopped early and these are the (still valid) partial results.
    const doneUnlisten = await listen<{ results: Layout[]; cancelled: boolean }>(
      "generate-done",
      (e) => {
        setResults(e.payload.results);
        initSaveStates(e.payload.results);
        setWasCancelled(e.payload.cancelled);
        setVisibleCount(10);
        setRunning(false);
        setCancelling(false);
        setProgress(null);
        clearListeners();
      },
    );

    unlisteners = [progressUnlisten, doneUnlisten];

    try {
      await startGenerate(baseName(), count(), pins(), algorithm());
    } catch (e) {
      setError(String(e));
      setRunning(false);
      setCancelling(false);
      setProgress(null);
      clearListeners();
    }
  }

  // The run keeps going until the current batch finishes; the UI stays in a
  // "cancelling" state until the backend's done event arrives.
  async function handleCancel() {
    setCancelling(true);
    await cancelGenerate().catch(console.error);
  }

  function updateSaveName(i: number, name: string) {
    setSaveStates((prev) => prev.map((s, idx) => (idx === i ? { ...s, name } : s)));
  }

  async function handleSave(i: number) {
    const state = saveStates()[i];
    try {
      await saveGenerated(i, state.name || undefined);
      setSaveStates((prev) => prev.map((s, idx) => (idx === i ? { ...s, saved: true } : s)));
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <div class="flex-1 min-h-0 overflow-y-auto flex flex-col gap-4 max-w-3xl">
      <h1 class="text-lg font-mono text-neutral-300">Generate / Improve</h1>

      {/* ── Form ──────────────────────────────────────────────── */}
      <div class="border border-neutral-700 p-4 flex flex-col gap-4">
        {/* Base layout */}
        <div class="flex items-center gap-3">
          <label class="text-sm text-neutral-400 font-mono w-28 shrink-0">Based on</label>
          <LayoutSearch value={baseName()} onSelect={setBaseName} />
          <span class="text-xs text-neutral-400 font-mono">{baseName()}</span>
        </div>

        {/* Count */}
        <div class="flex items-center gap-3">
          <label class="text-sm text-neutral-400 font-mono w-28 shrink-0">Count</label>
          <input
            type="number"
            class="bg-neutral-800 border border-neutral-600 text-neutral-100 font-mono text-sm px-2 py-1 w-28"
            value={countStr()}
            min={1}
            max={10000}
            onInput={(e) => setCountStr(e.currentTarget.value)}
          />
          <span class="text-xs text-neutral-500">500–1000 recommended</span>
        </div>

        {/* Algorithm */}
        <div class="flex items-center gap-3">
          <label class="text-sm text-neutral-400 font-mono w-28 shrink-0">Algorithm</label>
          <Dropdown value={algorithm()} onChange={setAlgorithm}>
            <For each={ALGORITHMS}>{(a) => <option value={a.id}>{a.label}</option>}</For>
          </Dropdown>
          <span class="text-xs text-neutral-500">
            {ALGORITHMS.find((a) => a.id === algorithm())?.hint}
          </span>
        </div>

        {/* Pins — click keys on the visual keyboard to pin them */}
        <div class="flex items-start gap-3">
          <label class="text-sm text-neutral-400 font-mono w-28 shrink-0 pt-1">Pins</label>
          <div class="flex flex-col gap-2">
            <div class="text-xs text-neutral-500">
              Click keys to pin them. Pinned keys (⚑) won't move during generation.
            </div>
            <Show when={baseLayout()}>
              {(bl) => (
                <div class="w-96">
                  <KeyboardDisplay
                    keys={bl().keys}
                    keyboard={bl().keyboard}
                    shape={bl().shape}
                    heatmap={appStore.charFrequencies}
                    interactive={true}
                    pinned={pinnedChars()}
                    onKeyClick={(ch) => togglePin(ch)}
                  />
                </div>
              )}
            </Show>
            <Show when={pinnedChars().size > 0}>
              <div class="flex items-center gap-2 text-xs font-mono text-neutral-400">
                <span>Pinned:</span>
                <span class="text-neutral-200">{[...pinnedChars()].join(" ")}</span>
                <button
                  class="border border-neutral-600 px-1.5 py-0.5 hover:bg-neutral-700 text-neutral-400"
                  onClick={() => setPinnedChars(new Set())}
                >
                  clear
                </button>
              </div>
            </Show>
          </div>
        </div>

        {/* Buttons */}
        <div class="flex gap-3 pt-1">
          <button
            class="border border-neutral-500 font-mono text-sm px-5 py-1.5 hover:bg-neutral-700 disabled:opacity-40"
            disabled={running()}
            onClick={handleGenerate}
          >
            {running() ? "Running…" : "Generate"}
          </button>
          <Show when={running()}>
            <button
              class="border border-red-700 text-red-400 font-mono text-sm px-5 py-1.5 hover:bg-red-900 disabled:opacity-40"
              disabled={cancelling()}
              onClick={handleCancel}
            >
              {cancelling() ? "Cancelling…" : "Cancel"}
            </button>
          </Show>
        </div>
      </div>

      <Show when={error()}>
        <div class="text-red-400 text-sm font-mono">{error()}</div>
      </Show>

      <Show when={wasCancelled() && results().length === 0}>
        <div class="text-yellow-500 text-sm font-mono">
          Cancelled before any layouts were finished.
        </div>
      </Show>

      {/* ── Progress ───────────────────────────────────────── */}
      <Show when={running() && progress()}>
        {(p) => (
          <div class="border border-neutral-700 p-4 font-mono text-sm text-neutral-400 flex flex-col gap-2">
            <div>
              Optimizing… {p().done} / {p().total}
            </div>
            <div class="h-1 bg-neutral-700 w-full">
              <div
                class="h-1 bg-neutral-400 transition-all"
                style={{
                  width: `${Math.min(100, (p().done / p().total) * 100).toFixed(1)}%`,
                }}
              />
            </div>
          </div>
        )}
      </Show>

      {/* ── Results ───────────────────────────────────────────── */}
      <Show when={results().length > 0}>
        <div class="flex flex-col gap-1">
          <div class="text-xs text-neutral-500 font-mono uppercase tracking-widest mb-1">
            Results — top {results().length}
            <Show when={wasCancelled()}>
              <span class="text-yellow-500 normal-case tracking-normal ml-2">
                (cancelled — showing what was generated)
              </span>
            </Show>
          </div>

          <For each={results().slice(0, visibleCount())}>
            {(layout, i) => {
              const save = () => saveStates()[i()];
              return (
                <div class="border border-neutral-700 p-3 flex flex-col gap-3">
                  <div class="flex items-baseline gap-3 font-mono text-sm">
                    <span class="text-neutral-500 w-5 text-right shrink-0">#{i() + 1}</span>
                    <span class="text-neutral-300">score: {layout.stats.score.toFixed(5)}</span>
                    <span class="text-neutral-500 text-xs">
                      sfb: {layout.stats.sfb.toFixed(3)}%
                    </span>
                  </div>

                  <div class="pl-8 w-96">
                    <KeyboardDisplay
                      keys={layout.keys}
                      keyboard={layout.keyboard}
                      shape={layout.shape}
                      heatmap={appStore.charFrequencies}
                    />
                  </div>

                  <div class="pl-8 flex items-center gap-2">
                    <Show
                      when={!save()?.saved}
                      fallback={
                        <span class="text-xs font-mono text-neutral-400">
                          Saved as <span class="text-neutral-200">{save()?.name}</span>
                        </span>
                      }
                    >
                      <label class="text-xs text-neutral-500 font-mono shrink-0">Save as</label>
                      <input
                        class="bg-neutral-800 border border-neutral-600 text-neutral-100 font-mono text-xs px-2 py-0.5 w-44"
                        placeholder="name (optional)"
                        value={save()?.name ?? ""}
                        onInput={(e) => updateSaveName(i(), e.currentTarget.value)}
                      />
                      <button
                        class="border border-neutral-600 font-mono text-xs px-3 py-0.5 hover:bg-neutral-700"
                        onClick={() => handleSave(i())}
                      >
                        Save
                      </button>
                    </Show>
                  </div>
                </div>
              );
            }}
          </For>

          <Show when={visibleCount() < results().length}>
            <button
              class="border border-neutral-700 font-mono text-sm px-4 py-2 hover:bg-neutral-800 text-neutral-400 mt-1"
              onClick={() => setVisibleCount((n) => Math.min(n + 40, results().length))}
            >
              Show more ({results().length - visibleCount()} remaining)
            </button>
          </Show>
        </div>
      </Show>
    </div>
  );
}
