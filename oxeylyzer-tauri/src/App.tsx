import { createSignal, Match, Switch } from "solid-js";
import TitleBar from "./components/TitleBar";
import LayoutsView from "./views/LayoutsView";
import AnalyzeView from "./views/AnalyzeView";
import CompareView from "./views/CompareView";
import GenerateView from "./views/GenerateView";
import LanguageView from "./views/LanguageView";

type View = "layouts" | "analyze" | "compare" | "generate" | "language";

const NAV_ITEMS: { id: View; label: string }[] = [
    { id: "layouts", label: "Layouts" },
    { id: "analyze", label: "Analyze" },
    { id: "compare", label: "Compare" },
    { id: "generate", label: "Generate" },
    { id: "language", label: "Language" },
];

function App() {
    const [view, setView] = createSignal<View>("layouts");
    const [analyzeTarget, setAnalyzeTarget] = createSignal<string | undefined>(undefined);

    function goAnalyze(layoutName: string) {
        setAnalyzeTarget(layoutName);
        setView("analyze");
    }

    return (
        <div class="flex flex-col h-screen w-screen overflow-hidden bg-neutral-900 text-neutral-100 font-mono">
            <TitleBar />
            <div class="flex flex-1 min-h-0 overflow-hidden">
                {/* ── Sidebar ─────────────────────────────────────── */}
                <nav class="w-36 shrink-0 border-r border-neutral-700 flex flex-col pt-3 gap-0.5">
                    <div class="text-xs text-neutral-500 uppercase tracking-widest px-3 pb-2">
                        Oxeylyzer
                    </div>
                    {NAV_ITEMS.map((item) => (
                        <button
                            class="text-left text-sm px-3 py-2 hover:bg-neutral-800 border-l-2"
                            classList={{
                                "border-neutral-100 text-neutral-100 bg-neutral-800":
                                    view() === item.id,
                                "border-transparent text-neutral-400": view() !== item.id,
                            }}
                            onClick={() => setView(item.id)}
                        >
                            {item.label}
                        </button>
                    ))}
                </nav>

                {/* ── Main content ────────────────────────────────── */}
                <main class="flex-1 overflow-hidden flex flex-col p-4">
                    <Switch>
                        <Match when={view() === "layouts"}>
                            <LayoutsView onAnalyze={goAnalyze} />
                        </Match>
                        <Match when={view() === "analyze"}>
                            <AnalyzeView initialLayout={analyzeTarget()} />
                        </Match>
                        <Match when={view() === "compare"}>
                            <CompareView />
                        </Match>
                        <Match when={view() === "generate"}>
                            <GenerateView />
                        </Match>
                        <Match when={view() === "language"}>
                            <LanguageView />
                        </Match>
                    </Switch>
                </main>
            </div>
        </div>
    );
}

export default App;
