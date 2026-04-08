import { createSignal, For, Show } from "solid-js";
import KeyboardDisplay from "../components/KeyboardDisplay";
import { MOCK_LAYOUTS, MOCK_GENERATED_LAYOUTS } from "../mock";
import type { Layout } from "../mock";

type SaveState = { name: string; saved: boolean };

export default function GenerateView() {
    const [baseName, setBaseName] = createSignal(MOCK_LAYOUTS[0].name);
    const [count, setCount] = createSignal(1000);
    const [pins, setPins] = createSignal("");
    const [running, setRunning] = createSignal(false);
    const [results, setResults] = createSignal<Layout[]>([]);
    const [saveStates, setSaveStates] = createSignal<SaveState[]>([]);

    const initSaveStates = (layouts: Layout[]) =>
        layouts.map((l) => ({ name: l.name, saved: false }));

    function handleGenerate() {
        setRunning(true);
        setResults([]);
        // Simulate async work: just show results immediately in the wireframe
        setTimeout(() => {
            setResults(MOCK_GENERATED_LAYOUTS);
            setSaveStates(initSaveStates(MOCK_GENERATED_LAYOUTS));
            setRunning(false);
        }, 600);
    }

    function handleImprove() {
        setRunning(true);
        setResults([]);
        setTimeout(() => {
            // Improve produces slightly different mock results
            const improved = MOCK_GENERATED_LAYOUTS.map((l, i) => ({
                ...l,
                name: `improved-${i + 1}`,
                stats: { ...l.stats, score: l.stats.score + 0.01 * (5 - i) },
            }));
            setResults(improved);
            setSaveStates(initSaveStates(improved));
            setRunning(false);
        }, 600);
    }

    function updateSaveName(i: number, name: string) {
        setSaveStates((prev) => prev.map((s, idx) => (idx === i ? { ...s, name } : s)));
    }

    function handleSave(i: number) {
        setSaveStates((prev) => prev.map((s, idx) => (idx === i ? { ...s, saved: true } : s)));
    }

    return (
        <div class="flex-1 min-h-0 overflow-y-auto flex flex-col gap-4 max-w-3xl">
            <h1 class="text-lg font-mono text-neutral-300">Generate / Improve</h1>

            {/* ── Form ──────────────────────────────────────────────── */}
            <div class="border border-neutral-700 p-4 flex flex-col gap-4">
                {/* Base layout */}
                <div class="flex items-center gap-3">
                    <label class="text-sm text-neutral-400 font-mono w-28 shrink-0">Based on</label>
                    <select
                        class="bg-neutral-800 border border-neutral-600 text-neutral-100 font-mono text-sm px-2 py-1"
                        value={baseName()}
                        onChange={(e) => setBaseName(e.currentTarget.value)}
                    >
                        {MOCK_LAYOUTS.map((l) => (
                            <option value={l.name}>{l.name}</option>
                        ))}
                    </select>
                </div>

                {/* Count */}
                <div class="flex items-center gap-3">
                    <label class="text-sm text-neutral-400 font-mono w-28 shrink-0">Count</label>
                    <input
                        type="number"
                        class="bg-neutral-800 border border-neutral-600 text-neutral-100 font-mono text-sm px-2 py-1 w-28"
                        value={count()}
                        min={1}
                        max={10000}
                        onInput={(e) => setCount(parseInt(e.currentTarget.value, 10) || 1000)}
                    />
                    <span class="text-xs text-neutral-500">500–1000 recommended</span>
                </div>

                {/* Pins */}
                <div class="flex items-start gap-3">
                    <label class="text-sm text-neutral-400 font-mono w-28 shrink-0 pt-1">
                        Pins
                    </label>
                    <div class="flex flex-col gap-1 flex-1">
                        <input
                            class="bg-neutral-800 border border-neutral-600 text-neutral-100 font-mono text-sm px-2 py-1 w-48"
                            placeholder="e.g. euaoi"
                            value={pins()}
                            onInput={(e) => setPins(e.currentTarget.value)}
                        />
                        <Show when={pins().length > 0}>
                            <div class="text-xs text-neutral-500">
                                Pinning {pins().length} key{pins().length !== 1 ? "s" : ""}:{" "}
                                <span class="font-mono text-neutral-300">
                                    {pins().split("").join(" ")}
                                </span>
                            </div>
                        </Show>
                        <div class="text-xs text-neutral-600">
                            Pin characters to fixed positions. Useful for keeping vowel blocks in
                            place during improve.
                        </div>
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
                    <button
                        class="border border-neutral-500 font-mono text-sm px-5 py-1.5 hover:bg-neutral-700 disabled:opacity-40"
                        disabled={running()}
                        onClick={handleImprove}
                    >
                        {running() ? "Running…" : "Improve"}
                    </button>
                    <div class="flex flex-col justify-center text-xs text-neutral-600 font-mono ml-2">
                        <span>Generate = random new layouts</span>
                        <span>Improve = optimize the base layout</span>
                    </div>
                </div>
            </div>

            {/* ── Results ───────────────────────────────────────────── */}
            <Show when={running()}>
                <div class="border border-neutral-700 p-4 font-mono text-sm text-neutral-400">
                    Optimizing {count()} variants…
                </div>
            </Show>

            <Show when={results().length > 0}>
                <div class="flex flex-col gap-1">
                    <div class="text-xs text-neutral-500 font-mono uppercase tracking-widest mb-1">
                        Results — top {results().length}
                    </div>

                    <For each={results()}>
                        {(layout, i) => {
                            const save = () => saveStates()[i()];
                            return (
                                <div class="border border-neutral-700 p-3 flex flex-col gap-3">
                                    {/* Header */}
                                    <div class="flex items-baseline gap-3 font-mono text-sm">
                                        <span class="text-neutral-500 w-5 text-right shrink-0">
                                            #{i() + 1}
                                        </span>
                                        <span class="text-neutral-300">
                                            score: {layout.stats.score.toFixed(5)}
                                        </span>
                                        <span class="text-neutral-500 text-xs">
                                            sfb: {layout.stats.sfb.toFixed(3)}%
                                        </span>
                                    </div>

                                    {/* Keyboard */}
                                    <div class="pl-8">
                                        <KeyboardDisplay keys={layout.keys} />
                                    </div>

                                    {/* Save controls */}
                                    <div class="pl-8 flex items-center gap-2">
                                        <Show
                                            when={!save()?.saved}
                                            fallback={
                                                <span class="text-xs font-mono text-neutral-400">
                                                    Saved as{" "}
                                                    <span class="text-neutral-200">
                                                        {save()?.name}
                                                    </span>
                                                </span>
                                            }
                                        >
                                            <label class="text-xs text-neutral-500 font-mono shrink-0">
                                                Save as
                                            </label>
                                            <input
                                                class="bg-neutral-800 border border-neutral-600 text-neutral-100 font-mono text-xs px-2 py-0.5 w-44"
                                                placeholder="name (optional)"
                                                value={save()?.name ?? ""}
                                                onInput={(e) =>
                                                    updateSaveName(i(), e.currentTarget.value)
                                                }
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
                </div>
            </Show>
        </div>
    );
}
