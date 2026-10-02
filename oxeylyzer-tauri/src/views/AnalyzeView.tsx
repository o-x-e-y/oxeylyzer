import { batch, createEffect, createSignal, For, on, Show, untrack } from "solid-js";
import KeyboardDisplay from "../components/KeyboardDisplay";
import BigramList from "../components/BigramList";
import LayoutSearch from "../components/LayoutSearch";
import FingerStats from "../components/FingerStats";
import { AnalyzeStatColumns } from "../components/StatColumns";
import { NGRAM_TABS, type Layout, type BigramEntry, type LayoutStats } from "../types";
import { appStore, dataVersion, heatScheme, setHeatScheme, type HeatScheme } from "../store";
import { heatStyleFor } from "../heat";
import Dropdown from "../components/Dropdown";
import { analyzeLayout, getBigrams, getTrigrams, analyzeCustom, saveCustomLayout } from "../api";

type Props = {
  /** Layout to show; a new object reloads it even if it's already shown. */
  request?: { name: string };
  onEdit?: (_layoutName: string) => void;
  onLayoutShown?: (_layoutName: string) => void;
};

export default function AnalyzeView(props: Props) {
  // The saved layout being worked on, and the arrangement currently on screen.
  // The arrangement updates immediately; stats follow once the backend answers.
  const [base, setBase] = createSignal<Layout | null>(null);
  const [keys, setKeys] = createSignal("");
  // Disabled positions move with their keys when keys are swapped.
  const [disabled, setDisabled] = createSignal<Set<number>>(new Set());
  const [stats, setStats] = createSignal<LayoutStats | null>(null);
  const [previousStats, setPreviousStats] = createSignal<LayoutStats | null>(null);

  const [activeTabId, setActiveTabId] = createSignal<string>("sfbs");
  const activeTab = () => NGRAM_TABS.find((t) => t.id === activeTabId()) ?? NGRAM_TABS[0];
  const [count, setCount] = createSignal(10);
  const [bigramData, setBigramData] = createSignal<BigramEntry[]>([]);
  const [highlightedKeys, setHighlightedKeys] = createSignal<string[]>([]);
  const [loading, setLoading] = createSignal(false);
  const [error, setError] = createSignal("");
  const [saveName, setSaveName] = createSignal("");
  const [saveMsg, setSaveMsg] = createSignal<{ text: string; ok: boolean } | null>(null);

  const isModified = () => {
    const b = base();
    return !!b && (keys() !== b.keys || disabled().size > 0);
  };
  const displayName = () => (base()?.name ?? "") + (isModified() ? "*" : "");

  // Sequence counter: any stale async result whose seq < current is discarded.
  // This prevents fast toggles / rapid selects from overwriting newer results.
  let seq = 0;
  const nextSeq = () => ++seq;
  const applyIfCurrent = (s: number, apply: () => void) => {
    if (seq === s) apply();
  };

  async function run(s: number, work: () => Promise<void>) {
    setLoading(true);
    setError("");
    try {
      await work();
    } catch (e) {
      applyIfCurrent(s, () => setError(String(e)));
    } finally {
      applyIfCurrent(s, () => setLoading(false));
    }
  }

  /** Shows a saved layout, dropping swaps and disabled keys. */
  function show(name: string) {
    const s = nextSeq();
    return run(s, async () => {
      const l = await analyzeLayout(name);
      applyIfCurrent(s, () => {
        batch(() => {
          setBase(l);
          setKeys(l.keys);
          setDisabled(new Set<number>());
          setStats(l.stats);
          setPreviousStats(null);
          setSaveMsg(null);
        });
        props.onLayoutShown?.(l.name);
      });
    });
  }

  /** Re-analyzes the arrangement on screen; `fromRefresh` drops the delta baseline. */
  function analyzeCurrent(fromRefresh = false) {
    const b = base();
    if (!b) return;
    const s = nextSeq();
    const before = stats();
    return run(s, async () => {
      const fresh = fromRefresh ? await analyzeLayout(b.name) : b;
      // The file changed underneath us — start over from what's saved now.
      if (fresh.keys !== b.keys) {
        applyIfCurrent(s, () => show(fresh.name));
        return;
      }
      const result = isModified() ? await analyzeCustom(b.name, keys(), [...disabled()]) : fresh;
      applyIfCurrent(s, () =>
        batch(() => {
          setBase(fresh);
          setStats(result.stats);
          setPreviousStats(fromRefresh ? null : before);
        }),
      );
    });
  }

  createEffect(
    on(
      () => props.request,
      (request) => {
        const name = request?.name ?? untrack(() => appStore.layouts[0]?.name);
        if (name) show(name);
      },
    ),
  );

  // Config, language and layout-file changes all bump dataVersion.
  createEffect(on(dataVersion, () => analyzeCurrent(true), { defer: true }));

  // Refetch ngram lists when the arrangement, disabled keys, or tab changes.
  // Modified arrangements pass their keys/disabled positions so the lists
  // describe what's on screen, not the original saved layout.
  let ngramSeq = 0;
  createEffect(() => {
    const b = base();
    const tab = activeTab();
    const k = keys();
    const d = [...disabled()];
    dataVersion();
    if (!b) return;

    const s = ++ngramSeq;
    const customKeys = k !== b.keys ? k : undefined;
    const disabledArg = d.length > 0 ? d : undefined;
    const request =
      tab.kind === "bigram"
        ? getBigrams(b.name, tab.id, 50, customKeys, disabledArg)
        : getTrigrams(b.name, tab.id, 50, customKeys, disabledArg).then((entries) =>
            entries.map((e) => ({ bigram: e.trigram, percent: e.percent })),
          );
    request
      .then((entries) => s === ngramSeq && setBigramData(entries))
      .catch((e) => s === ngramSeq && setError(String(e)));
  });

  function handleSwap(fromIdx: number, toIdx: number) {
    const arr = Array.from(keys());
    [arr[fromIdx], arr[toIdx]] = [arr[toIdx], arr[fromIdx]];
    const prev = disabled();
    const moved = new Set<number>();
    for (const idx of prev) moved.add(idx === fromIdx ? toIdx : idx === toIdx ? fromIdx : idx);
    batch(() => {
      setKeys(arr.join(""));
      setDisabled(moved);
    });
    analyzeCurrent();
  }

  function handleToggleDisabled(idx: number) {
    const next = new Set(disabled());
    if (next.has(idx)) next.delete(idx);
    else next.add(idx);
    setDisabled(next);
    analyzeCurrent();
  }

  function handleReset() {
    const b = base();
    if (b) show(b.name);
  }

  async function handleSaveAs() {
    const b = base();
    const name = saveName().trim();
    if (!b || !name) return;
    try {
      const saved = await saveCustomLayout(b.name, keys(), name);
      setSaveName("");
      await show(saved.name);
      setSaveMsg({ text: `Saved as "${saved.name}".`, ok: true });
    } catch (e) {
      setSaveMsg({ text: String(e), ok: false });
    }
  }

  const displayBigrams = () => bigramData().slice(0, count());
  const maxFreq = () => Math.max(1, ...Object.values(appStore.charFrequencies));
  const legendSteps = () => Array.from({ length: 13 }, (_, i) => (i / 12) * maxFreq());

  return (
    <div class="flex-1 min-h-0 overflow-y-auto flex flex-col gap-4">
      {/* ── Toolbar ─────────────────────────────────────────────── */}
      <div class="shrink-0 flex items-center gap-2 border border-neutral-700 p-2">
        <label class="text-neutral-400 text-sm shrink-0">Layout</label>
        <LayoutSearch value={base()?.name ?? ""} onSelect={show} />
        <Show when={loading()}>
          <span class="text-neutral-500 text-sm">…</span>
        </Show>
        <Show when={base()}>
          <span class="font-mono text-sm text-neutral-300">{displayName()}</span>
        </Show>
        <Show when={isModified()}>
          <button
            class="border border-neutral-600 text-xs font-mono px-2 py-0.5 hover:bg-neutral-700"
            onClick={handleReset}
          >
            ↩ Reset
          </button>
          <input
            class="bg-neutral-800 border border-neutral-600 text-neutral-100 font-mono text-xs px-2 py-0.5 w-32"
            placeholder="save as…"
            value={saveName()}
            onInput={(e) => setSaveName(e.currentTarget.value)}
            onKeyDown={(e) => e.key === "Enter" && saveName().trim() && handleSaveAs()}
          />
          <button
            class="border border-neutral-600 text-xs font-mono px-2 py-0.5 hover:bg-neutral-700 disabled:opacity-40"
            disabled={!saveName().trim()}
            onClick={handleSaveAs}
          >
            Save
          </button>
        </Show>
        <Show when={saveMsg()}>
          {(m) => (
            <span
              class="text-xs font-mono"
              classList={{ "text-neutral-400": m().ok, "text-red-400": !m().ok }}
            >
              {m().text}
            </span>
          )}
        </Show>
        <Show when={base() && !isModified()}>
          <button
            class="border border-neutral-600 px-3 py-1 text-sm hover:bg-neutral-700"
            onClick={() => props.onEdit?.(base()!.name)}
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

      <Show when={error()}>
        <div class="shrink-0 text-red-400 text-sm font-mono">{error()}</div>
      </Show>

      <Show when={base()}>
        {(b) => (
          <div class="flex flex-col gap-6">
            {/* ── Keyboard ─────────────────────────────────────── */}
            <div class="flex flex-col gap-2 w-96">
              <div class="font-mono text-neutral-200">{displayName()}</div>
              <KeyboardDisplay
                keys={keys()}
                keyboard={b().keyboard}
                shape={b().shape}
                heatmap={appStore.charFrequencies}
                highlight={highlightedKeys().length > 0 ? highlightedKeys() : undefined}
                draggable={true}
                onSwap={handleSwap}
                disabledIndices={disabled()}
                onToggleDisabled={handleToggleDisabled}
              />
              <div class="text-xs text-neutral-700 font-mono">
                drag to swap · right-click to disable
              </div>
              {/* heat legend for the active color scheme, over the corpus' frequency range */}
              <div class="flex items-center gap-0 font-mono text-[10px] text-neutral-500">
                <span class="mr-1.5">0%</span>
                <For each={legendSteps()}>
                  {(pct) => <div class="w-4 h-2.5" style={heatStyleFor(pct)} />}
                </For>
                <span class="ml-1.5">{maxFreq().toFixed(1)}%</span>
              </div>
            </div>

            {/* ── Stat columns + finger load ───────────────────── */}
            <Show when={stats()}>
              {(s) => (
                <>
                  <AnalyzeStatColumns stats={s()} baseline={previousStats() ?? undefined} />
                  <FingerStats stats={s()} />
                </>
              )}
            </Show>

            {/* ── Ngram tabs (bigram + trigram categories) ─────── */}
            <div class="flex flex-col border border-neutral-700">
              <div class="shrink-0 flex items-center border-b border-neutral-700 flex-wrap">
                {NGRAM_TABS.map((tab) => (
                  <button
                    class="px-3 py-2 text-sm border-r border-neutral-700 hover:bg-neutral-800"
                    classList={{
                      "bg-neutral-700 text-white": activeTabId() === tab.id,
                      "text-neutral-400": activeTabId() !== tab.id,
                    }}
                    onClick={() => setActiveTabId(tab.id)}
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
                    onInput={(e) => {
                      // Leave an empty or partial field alone while typing; blur restores it.
                      const n = parseInt(e.currentTarget.value);
                      if (n >= 1) setCount(Math.min(n, 50));
                    }}
                    onBlur={(e) => (e.currentTarget.value = String(count()))}
                  />
                </div>
              </div>
              <div class="p-3">
                <BigramList
                  entries={displayBigrams()}
                  columns={2}
                  unit={activeTabId() === "fspeed" || activeTabId() === "stretches" ? "" : "%"}
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
