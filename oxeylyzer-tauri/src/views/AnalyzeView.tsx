import { createSignal, Show } from "solid-js";
import KeyboardDisplay from "../components/KeyboardDisplay";
import StatsPanel from "../components/StatsPanel";
import BigramList from "../components/BigramList";
import { MOCK_LAYOUTS, BIGRAM_TABS, getBigramData, type BigramTab, type Layout } from "../mock";

type Props = {
    initialLayout?: string;
};

export default function AnalyzeView(props: Props) {
    const initial = () =>
        MOCK_LAYOUTS.find((l) => l.name === props.initialLayout) ?? MOCK_LAYOUTS[0];

    const [selectedName, setSelectedName] = createSignal(initial().name);
    const [nameOrNr, setNameOrNr] = createSignal("");
    const [layout, setLayout] = createSignal<Layout>(initial());
    const [activeTab, setActiveTab] = createSignal<BigramTab>("sfbs");
    const [count, setCount] = createSignal(10);
    const [swapInput, setSwapInput] = createSignal("");
    const [swapMsg, setSwapMsg] = createSignal("");

    function handleAnalyze() {
        const raw = nameOrNr().trim();
        const found = raw
            ? (MOCK_LAYOUTS.find((l) => l.name === raw) ?? MOCK_LAYOUTS[parseInt(raw, 10) - 1])
            : MOCK_LAYOUTS.find((l) => l.name === selectedName());
        if (found) {
            setLayout(found);
            setSelectedName(found.name);
            setSwapMsg("");
        }
    }

    function handleSwap() {
        if (!swapInput().trim()) return;
        setSwapMsg(`Swap applied: "${swapInput().trim()}" on ${layout().name}`);
    }

    const bigramData = () => getBigramData(activeTab()).slice(0, count());

    return (
        <div class="flex flex-col gap-4 h-full">
            {/* ── Toolbar ─────────────────────────────────────────── */}
            <div class="flex items-center gap-2 border border-neutral-700 p-2">
                <label class="text-neutral-400 text-sm shrink-0">Layout</label>
                <select
                    class="bg-neutral-800 border border-neutral-600 text-sm px-2 py-1"
                    value={selectedName()}
                    onChange={(e) => setSelectedName(e.currentTarget.value)}
                >
                    {MOCK_LAYOUTS.map((l) => (
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
                    class="border border-neutral-500 px-3 py-1 text-sm hover:bg-neutral-700"
                    onClick={handleAnalyze}
                >
                    Analyze
                </button>
            </div>

            {/* ── Main content ────────────────────────────────────── */}
            <div class="flex gap-4 flex-1 min-h-0">
                {/* Left column: keyboard + quick stats + swap */}
                <div class="flex flex-col gap-4 w-fit shrink-0">
                    {/* Layout name + keyboard */}
                    <div class="border border-neutral-700 p-3 flex flex-col gap-3">
                        <span class="font-mono text-base">{layout().name}</span>
                        <KeyboardDisplay keys={layout().keys} />
                    </div>

                    {/* Quick stats bar */}
                    <div class="border border-neutral-700 p-3 font-mono text-sm flex flex-col gap-1">
                        <div class="flex justify-between gap-6">
                            <span class="text-neutral-400">Sfb</span>
                            <span>{layout().stats.sfb.toFixed(3)}%</span>
                        </div>
                        <div class="flex justify-between gap-6">
                            <span class="text-neutral-400">Dsfb</span>
                            <span>{layout().stats.dsfb.toFixed(3)}%</span>
                        </div>
                        <div class="flex justify-between gap-6">
                            <span class="text-neutral-400">Fspeed</span>
                            <span>{layout().stats.fspeed.toFixed(3)}</span>
                        </div>
                        <div class="flex justify-between gap-6">
                            <span class="text-neutral-400">Score</span>
                            <span>{layout().stats.score.toFixed(3)}</span>
                        </div>
                    </div>

                    {/* Swap keys */}
                    <div class="border border-neutral-700 p-3 flex flex-col gap-2">
                        <div class="text-xs text-neutral-500 uppercase tracking-widest">
                            Swap Keys
                        </div>
                        <div class="text-xs text-neutral-500">
                            e.g. <span class="font-mono">ab</span> swaps a↔b,{" "}
                            <span class="font-mono">abc</span> cycles a→b→c
                        </div>
                        <div class="flex gap-2">
                            <input
                                class="bg-neutral-800 border border-neutral-600 text-sm px-2 py-1 font-mono flex-1"
                                placeholder="ab abc ..."
                                value={swapInput()}
                                onInput={(e) => setSwapInput(e.currentTarget.value)}
                                onKeyDown={(e) => e.key === "Enter" && handleSwap()}
                            />
                            <button
                                class="border border-neutral-500 px-3 py-1 text-sm hover:bg-neutral-700"
                                onClick={handleSwap}
                            >
                                Swap
                            </button>
                        </div>
                        <Show when={swapMsg()}>
                            <div class="text-xs text-neutral-400 font-mono">{swapMsg()}</div>
                        </Show>
                    </div>
                </div>

                {/* Right column: full stats */}
                <div class="flex-1 overflow-y-auto">
                    <StatsPanel stats={layout().stats} />
                </div>
            </div>

            {/* ── Bigram detail tabs ───────────────────────────────── */}
            <div class="border border-neutral-700 flex flex-col">
                {/* Tab bar */}
                <div class="flex items-center gap-0 border-b border-neutral-700">
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
                            onInput={(e) => setCount(parseInt(e.currentTarget.value, 10) || 10)}
                        />
                    </div>
                </div>

                {/* Tab content */}
                <div class="p-3">
                    <BigramList entries={bigramData()} />
                </div>
            </div>
        </div>
    );
}
