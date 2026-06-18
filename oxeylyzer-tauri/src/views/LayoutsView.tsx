import { createSignal, For, Show } from "solid-js";
import KeyboardDisplay from "../components/KeyboardDisplay";
import Dropdown from "../components/Dropdown";
import { appStore, initStore, refreshStore } from "../store";
import { setLanguage, deleteLayout } from "../api";

type Props = {
  onAnalyze?: (_layoutName: string) => void;
  onEdit?: (_layoutName: string) => void;
};

type SortKey = "score" | "sfb" | "dsfb" | "fspeed" | "scissors" | "lsbs" | "stretches";

const BOARD_TYPES = ["ortho", "ansi", "iso", "colstag", "rowstag"] as const;

const SORT_COLS: { key: SortKey; label: string }[] = [
  { key: "score", label: "Score" },
  { key: "sfb", label: "SFB%" },
  { key: "dsfb", label: "DSFB%" },
  { key: "fspeed", label: "Fspeed" },
  { key: "scissors", label: "Scissors" },
  { key: "lsbs", label: "LSBs" },
  { key: "stretches", label: "Stretches" },
];

function statForKey(stats: { [k: string]: number }, key: SortKey): number {
  return (stats as Record<string, number>)[key] ?? 0;
}

export default function LayoutsView(props: Props) {
  const [sortKey, setSortKey] = createSignal<SortKey>("score");
  const [sortAsc, setSortAsc] = createSignal(false);
  const [boardFilter, setBoardFilter] = createSignal<string | null>(null);
  const [expandedLayout, setExpandedLayout] = createSignal<string | null>(null);
  const [changingLanguage, setChangingLanguage] = createSignal(false);
  const [pendingLang, setPendingLang] = createSignal(appStore.currentLanguage || "english");
  const [confirmDelete, setConfirmDelete] = createSignal<string | null>(null);
  const [deleteError, setDeleteError] = createSignal("");

  async function handleDelete(name: string) {
    if (confirmDelete() !== name) {
      setConfirmDelete(name);
      return;
    }
    setConfirmDelete(null);
    setDeleteError("");
    try {
      await deleteLayout(name);
      if (expandedLayout() === name) setExpandedLayout(null);
      await refreshStore();
    } catch (e) {
      setDeleteError(String(e));
    }
  }

  const filtered = () => {
    const bf = boardFilter();
    if (!bf) return appStore.layouts;
    return appStore.layouts.filter((l) => l.board === bf);
  };

  const sorted = () => {
    const key = sortKey();
    const asc = sortAsc();
    return [...filtered()].sort((a, b) => {
      const va = key === "score" ? a.stats.score : statForKey(a.stats as any, key);
      const vb = key === "score" ? b.stats.score : statForKey(b.stats as any, key);
      // score: higher = better (desc by default); penalties: lower = better (asc by default)
      const natural = key === "score" ? vb - va : va - vb;
      return asc ? -natural : natural;
    });
  };

  const toggleSort = (key: SortKey) => {
    if (sortKey() === key) {
      setSortAsc((v) => !v);
    } else {
      setSortKey(key);
      setSortAsc(false);
    }
  };

  const toggleExpand = (name: string) => {
    setExpandedLayout((prev) => (prev === name ? null : name));
  };

  async function handleSetLanguage() {
    setChangingLanguage(true);
    try {
      await setLanguage(pendingLang());
      await initStore();
    } catch (e) {
      console.error("Failed to set language:", e);
    } finally {
      setChangingLanguage(false);
    }
  }

  return (
    <div class="flex-1 min-h-0 overflow-y-auto flex flex-col gap-4 max-w-3xl">
      <h1 class="text-lg font-mono font-bold">Layouts</h1>

      {/* Controls row */}
      <div class="flex flex-wrap gap-4 items-end border border-neutral-700 p-3">
        {/* Language picker */}
        <div class="flex flex-col gap-1">
          <label class="text-xs text-neutral-400 font-mono uppercase tracking-widest">
            Language
          </label>
          <div class="flex gap-2 items-center">
            <Dropdown value={pendingLang()} onChange={setPendingLang}>
              <For each={appStore.languages}>{(lang) => <option value={lang}>{lang}</option>}</For>
            </Dropdown>
            <button
              class="border border-neutral-600 font-mono text-sm px-2 py-1 hover:bg-neutral-700 disabled:opacity-40"
              disabled={changingLanguage()}
              onClick={handleSetLanguage}
            >
              {changingLanguage() ? "…" : "Set"}
            </button>
            <span class="text-xs text-neutral-500">current: {appStore.currentLanguage}</span>
          </div>
        </div>

        {/* Board type filter */}
        <div class="flex flex-col gap-1">
          <label class="text-xs text-neutral-400 font-mono uppercase tracking-widest">
            Board type
          </label>
          <div class="flex gap-1 flex-wrap">
            <button
              class="border font-mono text-xs px-2 py-1 hover:bg-neutral-700"
              classList={{
                "border-neutral-400 text-neutral-100 bg-neutral-700": boardFilter() === null,
                "border-neutral-600 text-neutral-400": boardFilter() !== null,
              }}
              onClick={() => setBoardFilter(null)}
            >
              all
            </button>
            <For each={BOARD_TYPES}>
              {(bt) => (
                <button
                  class="border font-mono text-xs px-2 py-1 hover:bg-neutral-700"
                  classList={{
                    "border-neutral-400 text-neutral-100 bg-neutral-700": boardFilter() === bt,
                    "border-neutral-600 text-neutral-400": boardFilter() !== bt,
                  }}
                  onClick={() => setBoardFilter(boardFilter() === bt ? null : bt)}
                >
                  {bt}
                </button>
              )}
            </For>
          </div>
        </div>

        {/* Sort columns */}
        <div class="flex flex-col gap-1">
          <label class="text-xs text-neutral-400 font-mono uppercase tracking-widest">
            Sort by
          </label>
          <div class="flex gap-1 flex-wrap">
            <For each={SORT_COLS}>
              {(col) => (
                <button
                  class="border font-mono text-xs px-2 py-1 hover:bg-neutral-700"
                  classList={{
                    "border-neutral-400 text-neutral-100 bg-neutral-700": sortKey() === col.key,
                    "border-neutral-600 text-neutral-400": sortKey() !== col.key,
                  }}
                  onClick={() => toggleSort(col.key)}
                >
                  {col.label}
                  {sortKey() === col.key ? (sortAsc() ? " ↑" : " ↓") : ""}
                </button>
              )}
            </For>
          </div>
        </div>
      </div>

      <Show when={deleteError()}>
        <div class="text-red-400 text-xs font-mono">{deleteError()}</div>
      </Show>

      {/* Layout list */}
      <div class="flex flex-col gap-2">
        <For each={sorted()}>
          {(layout, i) => (
            <div
              class="border border-neutral-700 p-3 flex flex-col gap-2 cursor-pointer hover:bg-neutral-800/40"
              onClick={() => toggleExpand(layout.name)}
            >
              {/* Header row */}
              <div class="flex items-baseline gap-3 font-mono">
                <span class="text-neutral-500 text-sm w-6 text-right">#{i() + 1}</span>
                <span class="text-neutral-100 font-bold">{layout.name}</span>
                <Show when={layout.board}>
                  <span class="text-neutral-600 text-xs">{layout.board}</span>
                </Show>
                <span class="text-neutral-400 text-sm">score: {layout.stats.score.toFixed(3)}</span>
                <Show
                  when={sortKey() !== "score" && sortKey() !== "sfb"}
                  fallback={
                    <span class="text-neutral-500 text-xs">
                      sfb: {layout.stats.sfb.toFixed(3)}%
                    </span>
                  }
                >
                  <span class="text-neutral-500 text-xs">
                    {SORT_COLS.find((c) => c.key === sortKey())?.label ?? sortKey()}:{" "}
                    {statForKey(layout.stats as any, sortKey()).toFixed(3)}
                    {sortKey() !== "fspeed" && sortKey() !== "stretches" ? "%" : ""}
                  </span>
                </Show>
                <div class="ml-auto flex gap-2" onClick={(e) => e.stopPropagation()}>
                  <button
                    class="border border-neutral-600 text-xs font-mono px-2 py-0.5 hover:bg-neutral-700"
                    onClick={() => toggleExpand(layout.name)}
                  >
                    {expandedLayout() === layout.name ? "▲ hide" : "▼ keys"}
                  </button>
                  <button
                    class="border border-neutral-600 text-xs font-mono px-2 py-0.5 hover:bg-neutral-700"
                    onClick={() => props.onAnalyze?.(layout.name)}
                  >
                    Analyze →
                  </button>
                  <button
                    class="border border-neutral-600 text-xs font-mono px-2 py-0.5 hover:bg-neutral-700"
                    onClick={() => props.onEdit?.(layout.name)}
                  >
                    Edit
                  </button>
                  <button
                    class="border text-xs font-mono px-2 py-0.5"
                    classList={{
                      "border-red-700 text-red-400 hover:bg-red-900":
                        confirmDelete() === layout.name,
                      "border-neutral-700 text-neutral-500 hover:bg-neutral-700 hover:text-red-400":
                        confirmDelete() !== layout.name,
                    }}
                    onClick={() => handleDelete(layout.name)}
                    onMouseLeave={() => setConfirmDelete(null)}
                  >
                    {confirmDelete() === layout.name ? "really delete?" : "Delete"}
                  </button>
                </div>
              </div>

              {/* Expanded keyboard */}
              <Show when={expandedLayout() === layout.name}>
                <div class="pt-1 pl-6 w-96">
                  <KeyboardDisplay
                    keys={layout.keys}
                    keyboard={layout.keyboard}
                    shape={layout.shape}
                    heatmap={appStore.charFrequencies}
                  />
                  <div class="mt-2 flex gap-4 text-xs font-mono text-neutral-400">
                    <span>inrolls: {layout.stats.inrolls.toFixed(1)}%</span>
                    <span>outrolls: {layout.stats.outrolls.toFixed(1)}%</span>
                    <span>alternates: {layout.stats.alternates.toFixed(1)}%</span>
                    <span>redirects: {layout.stats.redirects.toFixed(3)}%</span>
                  </div>
                </div>
              </Show>
            </div>
          )}
        </For>
      </div>
    </div>
  );
}
