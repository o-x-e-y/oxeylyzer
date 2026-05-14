import { createSignal, Match, onMount, onCleanup, Show, Switch } from "solid-js";
import { listen } from "@tauri-apps/api/event";
import TitleBar from "./components/TitleBar";
import LayoutsView from "./views/LayoutsView";
import AnalyzeView from "./views/AnalyzeView";
import CompareView from "./views/CompareView";
import GenerateView from "./views/GenerateView";
import LanguageView from "./views/LanguageView";
import EditView from "./views/EditView";
import ConfigView from "./views/ConfigView";
import { initStore, appStore } from "./store";
import { getSession, setSession } from "./api";

type View = "layouts" | "analyze" | "compare" | "generate" | "language" | "edit" | "config";

const NAV_ITEMS: { id: View; label: string }[] = [
  { id: "layouts", label: "Layouts" },
  { id: "analyze", label: "Analyze" },
  { id: "compare", label: "Compare" },
  { id: "generate", label: "Generate" },
  { id: "language", label: "Language" },
  { id: "edit", label: "Edit" },
  { id: "config", label: "Config" },
];

function App() {
  const [view, setView] = createSignal<View>("layouts");
  const [analyzeTarget, setAnalyzeTarget] = createSignal<string | undefined>(undefined);
  const [editTarget, setEditTarget] = createSignal<string | undefined>(undefined);

  onMount(async () => {
    await initStore();

    // Auto-reload when config/layout files change on disk
    const unlisten = await listen("config-reloaded", () => initStore());
    onCleanup(unlisten);

    try {
      const session = await getSession();
      if (session.view && NAV_ITEMS.some((n) => n.id === session.view)) {
        setView(session.view as View);
      }
      if (session.lastLayout) {
        setAnalyzeTarget(session.lastLayout);
      }
    } catch {
      // session restore is best-effort
    }
  });

  function goAnalyze(layoutName: string) {
    setAnalyzeTarget(layoutName);
    setView("analyze");
    persistSession("analyze", layoutName);
  }

  function goEdit(layoutName: string) {
    setEditTarget(layoutName);
    setView("edit");
  }

  function navigate(v: View) {
    setView(v);
    persistSession(v, analyzeTarget() ?? null);
  }

  function persistSession(v: string, lastLayout: string | null) {
    setSession({
      view: v,
      language: appStore.currentLanguage,
      lastLayout,
    }).catch(() => {});
  }

  return (
    <div class="flex flex-col h-screen w-screen overflow-hidden bg-neutral-900 text-neutral-100 font-mono">
      <TitleBar />
      <div class="flex flex-1 min-h-0 overflow-hidden">
        {/* ── Sidebar ─────────────────────────────────────── */}
        <nav class="w-36 shrink-0 border-r border-neutral-700 flex flex-col pt-3 gap-0.5">
          <div class="text-xs text-neutral-500 uppercase tracking-widest px-3 pb-2">Oxeylyzer</div>
          {NAV_ITEMS.map((item) => (
            <button
              class="text-left text-sm px-3 py-2 hover:bg-neutral-800 border-l-2"
              classList={{
                "border-neutral-100 text-neutral-100 bg-neutral-800": view() === item.id,
                "border-transparent text-neutral-400": view() !== item.id,
              }}
              onClick={() => navigate(item.id)}
            >
              {item.label}
            </button>
          ))}
        </nav>

        {/* ── Main content ────────────────────────────────── */}
        <main class="flex-1 overflow-hidden flex flex-col p-4">
          <Show when={appStore.loading}>
            <div class="flex-1 flex items-center justify-center text-neutral-500 text-sm font-mono">
              Loading…
            </div>
          </Show>
          <Show when={appStore.error}>
            <div class="flex-1 flex items-center justify-center text-red-400 text-sm font-mono">
              Error: {appStore.error}
            </div>
          </Show>
          <Show when={!appStore.loading && !appStore.error}>
            <Switch>
              <Match when={view() === "layouts"}>
                <LayoutsView onAnalyze={goAnalyze} onEdit={goEdit} />
              </Match>
              <Match when={view() === "analyze"}>
                <AnalyzeView initialLayout={analyzeTarget()} onEdit={goEdit} />
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
              <Match when={view() === "edit"}>
                <EditView layoutName={editTarget()} />
              </Match>
              <Match when={view() === "config"}>
                <ConfigView />
              </Match>
            </Switch>
          </Show>
        </main>
      </div>
    </div>
  );
}

export default App;
