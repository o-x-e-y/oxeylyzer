import { createEffect, createSignal, Show } from "solid-js";
import LayoutSearch from "../components/LayoutSearch";
import KeyboardDisplay from "../components/KeyboardDisplay";
import { CompareStatColumns } from "../components/StatColumns";
import { appStore } from "../store";
import { analyzeLayout } from "../api";
import type { Layout } from "../types";

export default function CompareView() {
  const [name1, setName1] = createSignal(appStore.layouts[0]?.name ?? "");
  const [name2, setName2] = createSignal(appStore.layouts[1]?.name ?? "");
  const [compared, setCompared] = createSignal<[Layout, Layout] | null>(null);
  const [loading, setLoading] = createSignal(false);
  const [error, setError] = createSignal("");

  createEffect(() => {
    const n1 = name1(),
      n2 = name2();
    if (!n1 || !n2) return;
    setLoading(true);
    setError("");
    Promise.all([analyzeLayout(n1), analyzeLayout(n2)])
      .then(([l1, l2]) => setCompared([l1, l2]))
      .catch((e) => setError(String(e)))
      .finally(() => setLoading(false));
  });

  return (
    <div class="flex-1 min-h-0 overflow-y-auto flex flex-col gap-4">
      <h1 class="text-lg font-mono text-neutral-300 shrink-0">Compare</h1>

      {/* ── Pickers ──────────────────────────────────────────── */}
      <div class="flex gap-3 items-end shrink-0">
        <div class="flex flex-col gap-1">
          <label class="text-xs text-neutral-500 font-mono uppercase tracking-widest">
            Layout 1
          </label>
          <LayoutSearch value={name1()} onSelect={setName1} />
        </div>
        <div class="flex flex-col gap-1">
          <label class="text-xs text-neutral-500 font-mono uppercase tracking-widest">
            Layout 2
          </label>
          <LayoutSearch value={name2()} onSelect={setName2} />
        </div>
        <Show when={loading()}>
          <span class="text-neutral-500 text-sm font-mono pb-1">…</span>
        </Show>
      </div>

      <Show when={error()}>
        <div class="text-red-400 text-sm font-mono">{error()}</div>
      </Show>

      {/* ── Compared content ─────────────────────────────────── */}
      <Show when={compared()} keyed>
        {([l1, l2]) => (
          <div class="flex flex-col gap-6">
            {/* Keyboards */}
            <div class="flex gap-8 items-start">
              <div class="flex flex-col gap-2 w-96">
                <div class="font-mono text-neutral-200">{l1.name}</div>
                <KeyboardDisplay
                  keys={l1.keys}
                  keyboard={l1.keyboard}
                  shape={l1.shape}
                  heatmap={appStore.charFrequencies}
                />
              </div>
              <div class="flex flex-col gap-2 w-96">
                <div class="font-mono text-neutral-200">{l2.name}</div>
                <KeyboardDisplay
                  keys={l2.keys}
                  keyboard={l2.keyboard}
                  shape={l2.shape}
                  heatmap={appStore.charFrequencies}
                />
              </div>
            </div>

            {/* Stats */}
            <CompareStatColumns
              s1={l1.stats}
              s2={l2.stats}
              name1={l1.name}
              name2={l2.name}
            />
          </div>
        )}
      </Show>
    </div>
  );
}
