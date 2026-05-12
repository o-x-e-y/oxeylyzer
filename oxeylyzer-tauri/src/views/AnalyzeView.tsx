import { createEffect, createSignal, Show } from "solid-js";
import KeyboardDisplay from "../components/KeyboardDisplay";
import StatsPanel from "../components/StatsPanel";
import BigramList from "../components/BigramList";
import { BIGRAM_TABS, type BigramTab, type Layout, type BigramEntry, type LayoutStats } from "../mock";
import { appStore } from "../store";
import { analyzeLayout, getBigrams, swapKeys } from "../api";

type Props = {
    initialLayout?: string;
    onEdit?: (layoutName: string) => void;
};

export default function AnalyzeView(props: Props) {
    const initialName = () =>
        props.initialLayout ?? appStore.layouts[0]?.name ?? "";

    const [selectedName, setSelectedName] = createSignal(initialName());
    const [nameOrNr, setNameOrNr] = createSignal("");
    const [layout, setLayout] = createSignal<Layout | null>(null);
    const [baseline, setBaseline] = createSignal<LayoutStats | null>(null);
    const [activeTab, setActiveTab] = createSignal<BigramTab>("sfbs");
    const [count, setCount] = createSignal(10);
    const [swapInput, setSwapInput] = createSignal("");
    const [swapMsg, setSwapMsg] = createSignal("");
    const [bigramData, setBigramData] = createSignal<BigramEntry[]>([]);
    const [highlightedKeys, setHighlightedKeys] = createSignal<string[]>([]);
    const [loading, setLoading] = createSignal(false);

    // Load the initial layout once the store has data.
    createEffect(() => {
        const name = initialName();
        if (!name || layout()) return;
        analyzeLayout(name).then((l) => {
            setLayout(l);
            setBaseline(l.stats);
            setSelectedName(l.name);
        });
    });

    // Refetch bigrams when layout name or tab changes.
    createEffect(() => {
        const name = layout()?.name;
        const tab = activeTab();
        if (!name || name.endsWith("*")) {
            // Swapped layout: use the original base name to fetch bigrams
            const base = name?.replace(/\*+$/, "");
            if (!base) return;
            getBigrams(base, tab, 50).then(setBigramData);
        } else if (name) {
            getBigrams(name, tab, 50).then(setBigramData);
        }
    });

    async function handleAnalyze() {
        const raw = nameOrNr().trim();
        let targetName: string;
        if (raw) {
            // Check if it looks like an index
            const idx = parseInt(raw, 10);
            if (!isNaN(idx)) {
                targetName = appStore.layouts[idx - 1]?.name ?? raw;
            } else {
                targetName = raw;
            }
        } else {
            targetName = selectedName();
        }
        if (!targetName) return;
        setLoading(true);
        try {
            const l = await analyzeLayout(targetName);
            setLayout(l);
            setBaseline(l.stats);
            setSelectedName(l.name);
            setSwapMsg("");
        } catch (e) {
            setSwapMsg(`Error: ${e}`);
        } finally {
            setLoading(false);
        }
    }

    async function handleSwap() {
        const input = swapInput().trim();
        if (!input || !layout()) return;
        const baseName = layout()!.name.replace(/\*+$/, "");
        setLoading(true);
        try {
            const l = await swapKeys(baseName, input);
            setLayout(l);
            setSwapMsg(`Swapped: "${input}"`);
        } catch (e) {
            setSwapMsg(`Error: ${e}`);
        } finally {
            setLoading(false);
        }
    }

    function handleUndo() {
        // Reload the baseline (pre-swap) state
        const bl = baseline();
        const l = layout();
        if (!bl || !l) return;
        const baseName = l.name.replace(/\*+$/, "");
        analyzeLayout(baseName).then((fresh) => {
            setLayout(fresh);
            setSwapMsg("");
        });
    }

    const displayBigrams = () => bigramData().slice(0, count());

    const statDiff = (): Partial<LayoutStats> | null => {
        const base = baseline();
        const current = layout()?.stats;
        if (!base || !current || !layout()?.name.endsWith("*")) return null;
        const diff: Partial<LayoutStats> = {};
        (Object.keys(base) as (keyof LayoutStats)[]).forEach((k) => {
            if (typeof base[k] === "number") {
                (diff as Record<string, number>)[k as string] =
                    (current[k] as number) - (base[k] as number);
            }
        });
        return diff;
    };

    return (
        <div class="flex-1 min-h-0 flex flex-col gap-3 overflow-hidden">
            {/* ── Toolbar ─────────────────────────────────────────────── */}
            <div class="shrink-0 flex items-center gap-2 border border-neutral-700 p-2">
                <label class="text-neutral-400 text-sm shrink-0">Layout</label>
                <select
                    class="bg-neutral-800 border border-neutral-600 text-sm px-2 py-1"
                    value={selectedName()}
                    onChange={(e) => setSelectedName(e.currentTarget.value)}
                >
                    {appStore.layouts.map((l) => (
                        <option value={l.name}>{l.name}</option>
                    ))}
                </select>

                <span class="text-neutral-600 text-sm">or</span>

                <input
                    class="bg-neutral-800 border border-neutral-600 text-sm px-2 py-1 w-40"
                    placeholder="name / index"
                    value={nameOrNr()}
                    onInput={(e) => setNameOrNr(e.currentTarget.value)}
                    onKeyDown={(e) => e.key === "Enter" && handleAnalyze()}
                />
                <button
                    class="border border-neutral-500 px-3 py-1 text-sm hover:bg-neutral-700 disabled:opacity-40"
                    disabled={loading()}
                    onClick={handleAnalyze}
                >
                    {loading() ? "…" : "Analyze"}
                </button>
                <Show when={layout() && !layout()!.name.endsWith("*")}>
                    <button
                        class="border border-neutral-600 px-3 py-1 text-sm hover:bg-neutral-700"
                        onClick={() => props.onEdit?.(layout()!.name)}
                    >
                        Edit
                    </button>
                </Show>
            </div>

            {/* ── Three columns ───────────────────────────────────────── */}
            <div class="flex gap-4 flex-1 min-h-0">
                {/* Col 1 — keyboard + swap ─────────────────────────────── */}
                <div class="shrink-0 flex flex-col gap-3">
                    <div class="border border-neutral-700 p-3 flex flex-col gap-3">
                        <span class="font-mono text-base">{layout()?.name ?? "—"}</span>
                        <Show when={layout()}>
                            {(l) => (
                                <KeyboardDisplay
                                    keys={l().keys}
                                    heatmap={appStore.charFrequencies}
                                    highlight={highlightedKeys().length > 0 ? highlightedKeys() : undefined}
                                />
                            )}
                        </Show>
                    </div>

                    {/* Swap Keys */}
                    <div class="border border-neutral-700 p-3 flex flex-col gap-2">
                        <div class="text-xs text-neutral-500 uppercase tracking-widest">
                            Swap Keys
                        </div>
                        <div class="text-xs text-neutral-500">
                            e.g.{" "}
                            <span class="font-mono text-neutral-400">ab</span> swaps a↔b,{" "}
                            <span class="font-mono text-neutral-400">abc</span> cycles a→b→c
                        </div>
                        <div class="flex gap-2">
                            <input
                                class="bg-neutral-800 border border-neutral-600 text-sm px-2 py-1 font-mono flex-1"
                                placeholder="ab abc …"
                                value={swapInput()}
                                onInput={(e) => setSwapInput(e.currentTarget.value)}
                                onKeyDown={(e) => e.key === "Enter" && handleSwap()}
                            />
                            <button
                                class="border border-neutral-500 px-3 py-1 text-sm hover:bg-neutral-700 shrink-0 disabled:opacity-40"
                                disabled={loading()}
                                onClick={handleSwap}
                            >
                                Swap
                            </button>
                        </div>
                        <Show when={layout()?.name.endsWith("*")}>
                            <button
                                class="border border-neutral-600 text-xs font-mono px-2 py-0.5 self-start hover:bg-neutral-700"
                                onClick={handleUndo}
                            >
                                ↩ Reset
                            </button>
                        </Show>
                        <Show when={swapMsg()}>
                            <div class="text-xs text-neutral-400 font-mono">{swapMsg()}</div>
                        </Show>
                    </div>

                    {/* Stat diff panel — shown after a swap */}
                    <Show when={statDiff()}>
                        {(diff) => (
                            <div class="border border-neutral-700 p-3 flex flex-col gap-1 text-xs font-mono">
                                <div class="text-neutral-500 uppercase tracking-widest mb-1">
                                    Δ vs baseline
                                </div>
                                {(
                                    [
                                        ["sfb", "SFB"],
                                        ["dsfb", "DSFB"],
                                        ["fspeed", "Fspeed"],
                                        ["scissors", "Scissors"],
                                        ["score", "Score"],
                                    ] as [keyof LayoutStats, string][]
                                ).map(([k, label]) => {
                                    const d = (diff() as Record<string, number>)[k as string] ?? 0;
                                    const better =
                                        k === "score" ? d > 0 : d < 0;
                                    return (
                                        <div class="flex gap-2 justify-between">
                                            <span class="text-neutral-500">{label}</span>
                                            <span
                                                class={
                                                    better ? "text-green-400" : "text-red-400"
                                                }
                                            >
                                                {d > 0 ? "+" : ""}
                                                {d.toFixed(3)}
                                            </span>
                                        </div>
                                    );
                                })}
                            </div>
                        )}
                    </Show>
                </div>

                {/* Col 2 — compact stats ───────────────────────────────── */}
                <div class="shrink-0 border border-neutral-700 p-3 overflow-y-auto">
                    <Show when={layout()}>
                        {(l) => <StatsPanel stats={l().stats} />}
                    </Show>
                </div>

                {/* Col 3 — bigram tabs ─────────────────────────────────── */}
                <div class="flex-1 flex flex-col border border-neutral-700 min-h-0">
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
                                onInput={(e) =>
                                    setCount(parseInt(e.currentTarget.value, 10) || 10)
                                }
                            />
                        </div>
                    </div>

                    <div class="flex-1 overflow-y-auto p-3">
                        <BigramList
                            entries={displayBigrams()}
                            columns={2}
                            onHoverBigram={(chars) => setHighlightedKeys(chars)}
                            onLeave={() => setHighlightedKeys([])}
                        />
                    </div>
                </div>
            </div>
        </div>
    );
}
