import { createEffect, createSignal, Show } from "solid-js";
import KeyboardDisplay from "../components/KeyboardDisplay";
import StatsPanel from "../components/StatsPanel";
import BigramList from "../components/BigramList";
import LayoutSearch from "../components/LayoutSearch";
import { BIGRAM_TABS, type BigramTab, type Layout, type BigramEntry, type LayoutStats } from "../mock";
import { appStore } from "../store";
import { analyzeLayout, getBigrams, swapKeys, analyzeWithDisabled } from "../api";

type Props = {
    initialLayout?: string;
    onEdit?: (layoutName: string) => void;
};

function delta(v1: number, v2: number, higherIsBetter = false): { text: string; color: string } {
    const d = v2 - v1;
    if (Math.abs(d) < 0.00005) return { text: "=", color: "text-neutral-500" };
    if (higherIsBetter)
        return d > 0
            ? { text: `▲ +${d.toFixed(3)}`, color: "text-green-400" }
            : { text: `▼ ${d.toFixed(3)}`, color: "text-red-400" };
    return d < 0
        ? { text: `▲ ${d.toFixed(3)}`, color: "text-green-400" }
        : { text: `▼ +${d.toFixed(3)}`, color: "text-red-400" };
}

function DRow(props: { label: string; v1: string; v2: string; d: { text: string; color: string } }) {
    return (
        <div class="flex gap-2 font-mono text-xs py-px">
            <span class="w-28 shrink-0 text-neutral-500">{props.label}</span>
            <span class="w-16 text-right shrink-0 text-neutral-400">{props.v1}</span>
            <span class="w-16 text-right shrink-0 text-neutral-100">{props.v2}</span>
            <span class={`w-20 text-right shrink-0 ${props.d.color}`}>{props.d.text}</span>
        </div>
    );
}

export default function AnalyzeView(props: Props) {
    const initialName = () =>
        props.initialLayout ?? appStore.layouts[0]?.name ?? "";

    const [layout, setLayout] = createSignal<Layout | null>(null);
    const [baseline, setBaseline] = createSignal<Layout | null>(null);
    const [activeTab, setActiveTab] = createSignal<BigramTab>("sfbs");
    const [count, setCount] = createSignal(10);
    const [bigramData, setBigramData] = createSignal<BigramEntry[]>([]);
    const [highlightedKeys, setHighlightedKeys] = createSignal<string[]>([]);
    const [loading, setLoading] = createSignal(false);
    const [disabledIndices, setDisabledIndices] = createSignal<Set<number>>(new Set());

    // Load the initial layout once the store has data.
    createEffect(() => {
        const name = initialName();
        if (!name || layout()) return;
        analyzeLayout(name).then((l) => {
            setLayout(l);
            setBaseline(l);
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
        setLoading(true);
        setDisabledIndices(new Set());
        try {
            const l = await analyzeLayout(name);
            setLayout(l);
            setBaseline(l);
        } catch (e) {
            console.error(e);
        } finally {
            setLoading(false);
        }
    }

    async function handleSwap(fromIdx: number, toIdx: number) {
        const l = layout();
        if (!l) return;
        const baseName = l.name.replace(/\*+$/, "");
        const fromChar = l.keys[fromIdx];
        const toChar = l.keys[toIdx];
        if (!fromChar || !toChar) return;
        setLoading(true);
        try {
            const updated = await swapKeys(baseName, fromChar + toChar);
            setLayout(updated);
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
        const newDisabled = new Set(disabledIndices());
        if (newDisabled.has(idx)) newDisabled.delete(idx);
        else newDisabled.add(idx);
        setDisabledIndices(newDisabled);
        if (newDisabled.size === 0) {
            const fresh = await analyzeLayout(baseName);
            setLayout(fresh);
        } else {
            const updated = await analyzeWithDisabled(baseName, [...newDisabled]);
            setLayout(updated);
        }
    }

    function handleReset() {
        const bl = baseline();
        if (!bl) return;
        const baseName = bl.name.replace(/\*+$/, "");
        setDisabledIndices(new Set());
        analyzeLayout(baseName).then((fresh) => {
            setLayout(fresh);
        });
    }

    const displayBigrams = () => bigramData().slice(0, count());
    const isModified = () => layout()?.name.endsWith("*") ?? false;
    const baselineStats = () => baseline()?.stats;
    const currentStats = () => layout()?.stats;

    const pct = (v: number) => `${v.toFixed(3)}%`;
    const num = (v: number) => v.toFixed(3);
    const sum = (...vs: number[]) => `${vs.reduce((a, b) => a + b, 0).toFixed(3)}%`;

    return (
        <div class="flex-1 min-h-0 flex flex-col gap-3 overflow-hidden">
            {/* ── Toolbar ─────────────────────────────────────────────── */}
            <div class="shrink-0 flex items-center gap-2 border border-neutral-700 p-2">
                <label class="text-neutral-400 text-sm shrink-0">Layout</label>
                <LayoutSearch
                    value={layout()?.name.replace(/\*+$/, "") ?? ""}
                    onSelect={handleSelect}
                />
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
            </div>

            {/* ── Three columns ───────────────────────────────────────── */}
            <div class="flex gap-4 flex-1 min-h-0">
                {/* Col 1 — keyboard ───────────────────────────────────── */}
                <div class="shrink-0 flex flex-col gap-3">
                    <div class="border border-neutral-700 p-3 flex flex-col gap-3">
                        <Show when={layout()}>
                            {(l) => (
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
                            )}
                        </Show>
                        <div class="text-xs text-neutral-600 font-mono">
                            drag to swap · right-click to disable
                        </div>
                    </div>

                    {/* Inline delta — shown after any swap or disable */}
                    <Show when={isModified() && baselineStats() && currentStats()}>
                        <div class="border border-neutral-700 p-3 flex flex-col gap-0.5 text-xs font-mono overflow-y-auto max-h-72">
                            <div class="flex gap-2 text-neutral-600 pb-1 mb-1 border-b border-neutral-700">
                                <span class="w-28 shrink-0" />
                                <span class="w-16 text-right shrink-0">before</span>
                                <span class="w-16 text-right shrink-0">after</span>
                                <span class="w-20 text-right shrink-0">Δ</span>
                            </div>
                            {(() => {
                                const b = baselineStats()!;
                                const c = currentStats()!;
                                return (
                                    <>
                                        <DRow label="Score" v1={num(b.score)} v2={num(c.score)} d={delta(b.score, c.score, true)} />
                                        <DRow label="SFB" v1={pct(b.sfb)} v2={pct(c.sfb)} d={delta(b.sfb, c.sfb)} />
                                        <DRow label="DSFB" v1={pct(b.dsfb)} v2={pct(c.dsfb)} d={delta(b.dsfb, c.dsfb)} />
                                        <DRow label="Fspeed" v1={num(b.fspeed)} v2={num(c.fspeed)} d={delta(b.fspeed, c.fspeed)} />
                                        <DRow label="Stretches" v1={num(b.stretches)} v2={num(c.stretches)} d={delta(b.stretches, c.stretches)} />
                                        <DRow label="Scissors" v1={pct(b.scissors)} v2={pct(c.scissors)} d={delta(b.scissors, c.scissors)} />
                                        <DRow label="LSBs" v1={pct(b.lsbs)} v2={pct(c.lsbs)} d={delta(b.lsbs, c.lsbs)} />
                                        <DRow label="Pinky-Ring" v1={pct(b.pinky_ring)} v2={pct(c.pinky_ring)} d={delta(b.pinky_ring, c.pinky_ring)} />
                                        <DRow label="Inrolls" v1={pct(b.inrolls)} v2={pct(c.inrolls)} d={delta(b.inrolls, c.inrolls, true)} />
                                        <DRow label="Outrolls" v1={pct(b.outrolls)} v2={pct(c.outrolls)} d={delta(b.outrolls, c.outrolls, true)} />
                                        <DRow label="Alternates" v1={pct(b.alternates)} v2={pct(c.alternates)} d={delta(b.alternates, c.alternates, true)} />
                                        <DRow label="Alt. SFS" v1={pct(b.alternates_sfs)} v2={pct(c.alternates_sfs)} d={delta(b.alternates_sfs, c.alternates_sfs, true)} />
                                        <DRow label="Redirects" v1={pct(b.redirects)} v2={pct(c.redirects)} d={delta(b.redirects, c.redirects)} />
                                        <DRow label="Bad Redir." v1={pct(b.bad_redirects)} v2={pct(c.bad_redirects)} d={delta(b.bad_redirects, c.bad_redirects)} />
                                        <DRow label="Bad SFBs" v1={pct(b.bad_sfbs)} v2={pct(c.bad_sfbs)} d={delta(b.bad_sfbs, c.bad_sfbs)} />
                                    </>
                                );
                            })()}
                        </div>
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
                                    setCount(Math.max(1, parseInt(e.currentTarget.value) || 1))
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
