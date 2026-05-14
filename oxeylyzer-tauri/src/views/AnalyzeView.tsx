import { createEffect, createMemo, createSignal, Show } from "solid-js";
import KeyboardDisplay from "../components/KeyboardDisplay";
import BigramList from "../components/BigramList";
import LayoutSearch from "../components/LayoutSearch";
import { AnalyzeStatColumns } from "../components/StatColumns";
import { BIGRAM_TABS, type BigramTab, type Layout, type BigramEntry } from "../types";
import { appStore, heatScheme, setHeatScheme, type HeatScheme } from "../store";
import Dropdown from "../components/Dropdown";
import { analyzeLayout, getBigrams, analyzeCustom } from "../api";

type Props = {
  initialLayout?: string;
  onEdit?: (_layoutName: string) => void;
};

export default function AnalyzeView(props: Props) {
  const initialName = () => props.initialLayout ?? appStore.layouts[0]?.name ?? "";

  const [layout, setLayout] = createSignal<Layout | null>(null);
  const [baseline, setBaseline] = createSignal<Layout | null>(null);
  const [previous, setPrevious] = createSignal<Layout | null>(null);
  const [activeTab, setActiveTab] = createSignal<BigramTab>("sfbs");
  const [count, setCount] = createSignal(10);
  const [bigramData, setBigramData] = createSignal<BigramEntry[]>([]);
  const [highlightedKeys, setHighlightedKeys] = createSignal<string[]>([]);
  const [loading, setLoading] = createSignal(false);
  // Disabled state tracks characters, not positions, so they follow keys through swaps.
  const [disabledChars, setDisabledChars] = createSignal<Set<string>>(new Set());
  // Derive the current disabled position indices from the current key arrangement.
  const disabledIndices = createMemo(() => {
    const chars = disabledChars();
    const keys = layout()?.keys ?? "";
    const result = new Set<number>();
    for (let i = 0; i < keys.length; i++) {
      if (chars.has(keys[i])) result.add(i);
    }
    return result;
  });

  // Sequence counter: any stale async result whose seq < current is discarded.
  // This prevents fast toggles / rapid selects from overwriting newer results.
  let seq = 0;
  const nextSeq = () => ++seq;
  const applyIfCurrent = (s: number, apply: () => void) => {
    if (seq === s) apply();
  };

  // Load layout whenever initialName changes (handles both first mount and
  // re-navigation to a different layout while the view is already active).
  createEffect(() => {
    const name = initialName();
    if (!name) return;
    const s = nextSeq();
    setDisabledChars(new Set());
    analyzeLayout(name).then((l) => {
      applyIfCurrent(s, () => {
        setLayout(l);
        setBaseline(l);
        setPrevious(null);
      });
    });
  });

  // Refetch bigrams when layout name or tab changes.
  createEffect(() => {
    const name = layout()?.name;
    const tab = activeTab();
    const base = name?.replace(/\*+$/, "");
    if (!base) return;
    getBigrams(base, tab, 50).then(setBigramData);
  });

  async function handleSelect(name: string) {
    const s = nextSeq();
    setLoading(true);
    setDisabledChars(new Set());
    try {
      const l = await analyzeLayout(name);
      applyIfCurrent(s, () => {
        setLayout(l);
        setBaseline(l);
        setPrevious(null);
      });
    } catch (e) {
      console.error(e);
    } finally {
      setLoading(false);
    }
  }

  // Returns disabled position indices for a given key arrangement and disabled char set.
  function disabledIdxsFor(chars: Set<string>, keys: string): number[] {
    const result: number[] = [];
    for (let i = 0; i < keys.length; i++) {
      if (chars.has(keys[i])) result.push(i);
    }
    return result;
  }

  // Use analyzeLayout when fully unmodified so layout().name stays clean (no "*").
  async function analyzeState(
    baseName: string,
    keys: string,
    disabledIdxs: number[],
  ): Promise<Layout> {
    const isOriginal = disabledIdxs.length === 0 && keys === (baseline()?.keys ?? "");
    return isOriginal ? analyzeLayout(baseName) : analyzeCustom(baseName, keys, disabledIdxs);
  }

  async function handleSwap(fromIdx: number, toIdx: number) {
    const l = layout();
    if (!l) return;
    const baseName = l.name.replace(/\*+$/, "");
    const arr = l.keys.split("");
    [arr[fromIdx], arr[toIdx]] = [arr[toIdx], arr[fromIdx]];
    const newKeys = arr.join("");
    // Recompute disabled indices against the new key arrangement so chars follow their keys.
    const newDisabledIdxs = disabledIdxsFor(disabledChars(), newKeys);
    const s = nextSeq();
    setLoading(true);
    try {
      const updated = await analyzeState(baseName, newKeys, newDisabledIdxs);
      applyIfCurrent(s, () => {
        setPrevious(l);
        setLayout(updated);
      });
    } catch (e) {
      console.error(e);
    } finally {
      setLoading(false);
    }
  }

  async function handleToggleDisabled(idx: number) {
    const l = layout();
    if (!l) return;
    const baseName = l.name.replace(/\*+$/, "");
    const char = l.keys[idx];
    if (!char) return;
    const newChars = new Set(disabledChars());
    if (newChars.has(char)) newChars.delete(char);
    else newChars.add(char);
    setDisabledChars(newChars);
    const s = nextSeq();
    try {
      const idxs = disabledIdxsFor(newChars, l.keys);
      const updated = await analyzeState(baseName, l.keys, idxs);
      applyIfCurrent(s, () => {
        setPrevious(l);
        setLayout(updated);
      });
    } catch (e) {
      console.error(e);
    }
  }

  function handleReset() {
    const bl = baseline();
    if (!bl) return;
    const baseName = bl.name.replace(/\*+$/, "");
    const s = nextSeq();
    setDisabledChars(new Set());
    analyzeLayout(baseName).then((fresh) =>
      applyIfCurrent(s, () => {
        setLayout(fresh);
        setPrevious(null);
      }),
    );
  }

  const displayBigrams = () => bigramData().slice(0, count());
  const isModified = () => layout()?.name.endsWith("*") ?? false;

  return (
    <div class="flex-1 min-h-0 overflow-y-auto flex flex-col gap-4">
      {/* ── Toolbar ─────────────────────────────────────────────── */}
      <div class="shrink-0 flex items-center gap-2 border border-neutral-700 p-2">
        <label class="text-neutral-400 text-sm shrink-0">Layout</label>
        <LayoutSearch value={layout()?.name.replace(/\*+$/, "") ?? ""} onSelect={handleSelect} />
        <Show when={loading()}>
          <span class="text-neutral-500 text-sm">…</span>
        </Show>
        <Show when={layout()}>
          <span class="font-mono text-sm text-neutral-300">{layout()!.name}</span>
        </Show>
        <Show when={isModified()}>
          <button
            class="border border-neutral-600 text-xs font-mono px-2 py-0.5 hover:bg-neutral-700"
            onClick={handleReset}
          >
            ↩ Reset
          </button>
        </Show>
        <Show when={layout() && !isModified()}>
          <button
            class="border border-neutral-600 px-3 py-1 text-sm hover:bg-neutral-700"
            onClick={() => props.onEdit?.(layout()!.name)}
          >
            Edit
          </button>
        </Show>
        <div class="ml-auto flex items-center gap-2">
          <label class="text-neutral-400 text-sm shrink-0">Colors</label>
          <Dropdown value={heatScheme()} onChange={(v) => setHeatScheme(v as HeatScheme)}>
            <option value="original">Original</option>
            <option value="playground">Playground</option>
            <option value="v2">v2</option>
          </Dropdown>
        </div>
      </div>

      <Show when={layout()}>
        {(l) => (
          <div class="flex flex-col gap-6">
            {/* ── Keyboard ─────────────────────────────────────── */}
            <div class="flex flex-col gap-2 w-96">
              <div class="font-mono text-neutral-200">{l().name}</div>
              <KeyboardDisplay
                keys={l().keys}
                keyboard={l().keyboard}
                shape={l().shape}
                heatmap={appStore.charFrequencies}
                highlight={highlightedKeys().length > 0 ? highlightedKeys() : undefined}
                draggable={true}
                onSwap={handleSwap}
                disabledIndices={disabledIndices()}
                onToggleDisabled={handleToggleDisabled}
              />
              <div class="text-xs text-neutral-700 font-mono">
                drag to swap · right-click to disable
              </div>
            </div>

            {/* ── Stat columns ─────────────────────────────────── */}
            <AnalyzeStatColumns stats={l().stats} baseline={previous()?.stats} />

            {/* ── Bigram tabs ───────────────────────────────────── */}
            <div class="flex flex-col border border-neutral-700">
              <div class="shrink-0 flex items-center border-b border-neutral-700">
                {BIGRAM_TABS.map((tab) => (
                  <button
                    class="px-4 py-2 text-sm border-r border-neutral-700 hover:bg-neutral-800"
                    classList={{
                      "bg-neutral-700 text-white": activeTab() === tab.id,
                      "text-neutral-400": activeTab() !== tab.id,
                    }}
                    onClick={() => setActiveTab(tab.id)}
                  >
                    {tab.label}
                  </button>
                ))}
                <div class="flex items-center gap-2 ml-auto px-3">
                  <label class="text-neutral-500 text-xs">Count</label>
                  <input
                    type="number"
                    class="bg-neutral-800 border border-neutral-600 text-sm px-2 py-1 w-16 text-right"
                    value={count()}
                    min={1}
                    max={50}
                    onInput={(e) => setCount(Math.max(1, parseInt(e.currentTarget.value) || 1))}
                  />
                </div>
              </div>
              <div class="p-3">
                <BigramList
                  entries={displayBigrams()}
                  columns={2}
                  onHoverBigram={(chars) => setHighlightedKeys(chars)}
                  onLeave={() => setHighlightedKeys([])}
                />
              </div>
            </div>
          </div>
        )}
      </Show>
    </div>
  );
}
