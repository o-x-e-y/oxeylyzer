import { createSignal, For, Show } from "solid-js";
import { MOCK_LANGUAGES, MOCK_LAYOUTS } from "../mock";
import KeyboardDisplay from "../components/KeyboardDisplay";

type Props = {
    onAnalyze?: (layoutName: string) => void;
};

export default function LayoutsView(props: Props) {
    const [language, setLanguage] = createSignal("english");
    const [includeInput, setIncludeInput] = createSignal("");
    const [included, setIncluded] = createSignal<string[]>([]);
    const [sortAsc, setSortAsc] = createSignal(false);
    const [expandedLayout, setExpandedLayout] = createSignal<string | null>(null);

    const sorted = () => {
        const layouts = [...MOCK_LAYOUTS];
        return sortAsc()
            ? layouts.sort((a, b) => a.stats.score - b.stats.score)
            : layouts.sort((a, b) => b.stats.score - a.stats.score);
    };

    const addIncluded = () => {
        const val = includeInput().trim();
        if (val && !included().includes(val)) {
            setIncluded((prev) => [...prev, val]);
        }
        setIncludeInput("");
    };

    const removeIncluded = (lang: string) => {
        setIncluded((prev) => prev.filter((l) => l !== lang));
    };

    const toggleExpand = (name: string) => {
        setExpandedLayout((prev) => (prev === name ? null : name));
    };

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
                    <select
                        class="bg-neutral-800 border border-neutral-600 text-neutral-100 font-mono text-sm px-2 py-1"
                        value={language()}
                        onChange={(e) => setLanguage(e.currentTarget.value)}
                    >
                        <For each={MOCK_LANGUAGES}>
                            {(lang) => <option value={lang}>{lang}</option>}
                        </For>
                    </select>
                </div>

                {/* Include other languages */}
                <div class="flex flex-col gap-1">
                    <label class="text-xs text-neutral-400 font-mono uppercase tracking-widest">
                        Include layouts from
                    </label>
                    <div class="flex gap-2 items-center">
                        <input
                            class="bg-neutral-800 border border-neutral-600 text-neutral-100 font-mono text-sm px-2 py-1 w-32"
                            placeholder="language..."
                            value={includeInput()}
                            onInput={(e) => setIncludeInput(e.currentTarget.value)}
                            onKeyDown={(e) => e.key === "Enter" && addIncluded()}
                        />
                        <button
                            class="border border-neutral-600 font-mono text-sm px-2 py-1 hover:bg-neutral-700"
                            onClick={addIncluded}
                        >
                            + Add
                        </button>
                    </div>
                    <Show when={included().length > 0}>
                        <div class="flex flex-wrap gap-1 mt-1">
                            <For each={included()}>
                                {(lang) => (
                                    <span class="border border-neutral-600 font-mono text-xs px-2 py-0.5 flex items-center gap-1">
                                        {lang}
                                        <button
                                            class="text-neutral-400 hover:text-neutral-100 ml-1"
                                            onClick={() => removeIncluded(lang)}
                                        >
                                            ×
                                        </button>
                                    </span>
                                )}
                            </For>
                        </div>
                    </Show>
                </div>

                {/* Sort toggle */}
                <div class="flex flex-col gap-1">
                    <label class="text-xs text-neutral-400 font-mono uppercase tracking-widest">
                        Sort
                    </label>
                    <button
                        class="border border-neutral-600 font-mono text-sm px-2 py-1 hover:bg-neutral-700 w-36 text-left"
                        onClick={() => setSortAsc((v) => !v)}
                    >
                        Score {sortAsc() ? "↑ asc" : "↓ desc"}
                    </button>
                </div>
            </div>

            {/* Layout list */}
            <div class="flex flex-col gap-2">
                <For each={sorted()}>
                    {(layout, i) => (
                        <div class="border border-neutral-700 p-3 flex flex-col gap-2">
                            {/* Header row */}
                            <div class="flex items-baseline gap-3 font-mono">
                                <span class="text-neutral-500 text-sm w-6 text-right">
                                    #{i() + 1}
                                </span>
                                <span class="text-neutral-100 font-bold">{layout.name}</span>
                                <span class="text-neutral-400 text-sm">
                                    score: {layout.stats.score.toFixed(3)}
                                </span>
                                <span class="text-neutral-500 text-xs">
                                    sfb: {layout.stats.sfb.toFixed(3)}%
                                </span>
                                <div class="ml-auto flex gap-2">
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
                                </div>
                            </div>

                            {/* Expanded keyboard */}
                            <Show when={expandedLayout() === layout.name}>
                                <div class="pt-1 pl-6">
                                    <KeyboardDisplay keys={layout.keys} />
                                    <div class="mt-2 flex gap-4 text-xs font-mono text-neutral-400">
                                        <span>inrolls: {layout.stats.inrolls.toFixed(1)}%</span>
                                        <span>outrolls: {layout.stats.outrolls.toFixed(1)}%</span>
                                        <span>
                                            alternates: {layout.stats.alternates.toFixed(1)}%
                                        </span>
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
