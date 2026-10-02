import { createEffect, createSignal, Match, onCleanup, onMount, Show, Switch } from "solid-js";
import type { JSX } from "solid-js";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import TitleBar from "./components/TitleBar";
import LayoutsView from "./views/LayoutsView";
import AnalyzeView from "./views/AnalyzeView";
import CompareView from "./views/CompareView";
import GenerateView from "./views/GenerateView";
import LanguageView from "./views/LanguageView";
import EditView from "./views/EditView";
import ConfigView from "./views/ConfigView";
import {
  appStore,
  backendError,
  fetchStore,
  heatScheme,
  refreshStore,
  setBackendError,
  setHeatScheme,
} from "./store";
import type { HeatScheme } from "./store";
import { backendStatus, getSession, setSession } from "./api";

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

type Download = { status: string; bytesDone?: number; bytesTotal?: number };

function downloadText(d: Download): string {
  const mb = (bytes = 0) => (bytes / 1e6).toFixed(1);
  switch (d.status) {
    case "connecting":
      return "Connecting to download the data files…";
    case "downloading":
      return `Downloading data files… ${mb(d.bytesDone)} / ${mb(d.bytesTotal)} MB`;
    case "extracting":
      return "Extracting data files…";
    default:
      return "Loading…";
  }
}

function App() {
  const [view, setView] = createSignal<View>("layouts");
  const [phase, setPhase] = createSignal<"starting" | "ready" | "failed">("starting");
  const [startError, setStartError] = createSignal("");
  const [download, setDownload] = createSignal<Download | null>(null);
  // Navigation requests are fresh objects, so asking for the layout a view
  // already shows still reloads it.
  const [analyzeRequest, setAnalyzeRequest] = createSignal<{ name: string }>();
  const [editRequest, setEditRequest] = createSignal<{ name: string }>();
  const [lastLayout, setLastLayout] = createSignal<string | null>(null);

  let started = false;
  async function start() {
    if (started) return;
    started = true;
    try {
      const status = await backendStatus();
      if (status.error) setBackendError(status.error);
      await fetchStore();
    } catch (e) {
      setStartError(String(e));
      setPhase("failed");
      return;
    }

    try {
      const session = await getSession();
      if (NAV_ITEMS.some((n) => n.id === session.view)) setView(session.view as View);
      const last = appStore.layouts.find(
        (l) => l.name.toLowerCase() === session.lastLayout?.toLowerCase(),
      );
      if (last) {
        setLastLayout(last.name);
        setAnalyzeRequest({ name: last.name });
        setEditRequest({ name: last.name });
      }
      if (session.heatScheme && ["original", "playground", "v2"].includes(session.heatScheme)) {
        setHeatScheme(session.heatScheme as HeatScheme);
      }
    } catch {
      // session restore is best-effort
    }
    setPhase("ready");
  }

  onMount(() => {
    const unlisteners: Promise<UnlistenFn>[] = [
      listen("config-reloaded", () => refreshStore()),
      listen("layouts-reloaded", () => refreshStore()),
      listen<string>("load-error", (e) => setBackendError(e.payload)),
      listen<Download>("download-progress", (e) => setDownload(e.payload)),
      listen("backend-ready", () => start()),
    ];
    onCleanup(() => unlisteners.forEach((u) => u.then((unlisten) => unlisten())));
    // The backend may have become ready before the listener was registered.
    Promise.all(unlisteners)
      .then(() => backendStatus())
      .then((s) => {
        if (s.ready) start();
      })
      .catch((e) => {
        setStartError(String(e));
        setPhase("failed");
      });
  });

  createEffect(() => {
    const session = { view: view(), lastLayout: lastLayout(), heatScheme: heatScheme() };
    if (phase() === "ready") setSession(session).catch(() => {});
  });

  function goAnalyze(name: string) {
    setAnalyzeRequest({ name });
    setView("analyze");
  }

  function goEdit(name: string) {
    setEditRequest({ name });
    setView("edit");
  }

  // Views stay mounted while hidden, so switching tabs keeps their state —
  // including a generation run that finishes while another tab is open.
  const Pane = (p: { id: View; children: JSX.Element }) => (
    <div class="flex-1 min-h-0 flex flex-col" classList={{ hidden: view() !== p.id }}>
      {p.children}
    </div>
  );

  return (
    <div class="flex flex-col h-screen w-screen overflow-hidden bg-neutral-900 text-neutral-100 font-mono">
      <TitleBar />
      <Show when={backendError()}>
        <div class="shrink-0 flex items-start gap-3 border-b border-red-900 bg-red-950/50 px-3 py-1.5 text-xs text-red-300">
          <span class="flex-1 whitespace-pre-wrap">{backendError()}</span>
          <button class="text-red-400 hover:text-red-200" onClick={() => setBackendError(null)}>
            ✕
          </button>
        </div>
      </Show>
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
              onClick={() => setView(item.id)}
            >
              {item.label}
            </button>
          ))}
        </nav>

        {/* ── Main content ────────────────────────────────── */}
        <main class="flex-1 overflow-hidden flex flex-col p-4">
          <Switch>
            <Match when={phase() === "starting"}>
              <div class="flex-1 flex flex-col items-center justify-center gap-3 text-neutral-500 text-sm">
                <span>{download() ? downloadText(download()!) : "Loading…"}</span>
                <Show when={download()?.bytesTotal}>
                  {(total) => (
                    <div class="h-1 w-64 bg-neutral-700">
                      <div
                        class="h-1 bg-neutral-400"
                        style={{ width: `${Math.min(100, ((download()?.bytesDone ?? 0) / total()) * 100)}%` }}
                      />
                    </div>
                  )}
                </Show>
              </div>
            </Match>
            <Match when={phase() === "failed"}>
              <div class="flex-1 flex items-center justify-center text-red-400 text-sm">
                Error: {startError()}
              </div>
            </Match>
            <Match when={phase() === "ready"}>
              <Pane id="layouts">
                <LayoutsView onAnalyze={goAnalyze} onEdit={goEdit} />
              </Pane>
              <Pane id="analyze">
                <AnalyzeView request={analyzeRequest()} onEdit={goEdit} onLayoutShown={setLastLayout} />
              </Pane>
              <Pane id="compare">
                <CompareView />
              </Pane>
              <Pane id="generate">
                <GenerateView />
              </Pane>
              <Pane id="language">
                <LanguageView />
              </Pane>
              <Pane id="edit">
                <EditView request={editRequest()} onLayoutShown={setLastLayout} />
              </Pane>
              <Pane id="config">
                <ConfigView />
              </Pane>
            </Match>
          </Switch>
        </main>
      </div>
    </div>
  );
}

export default App;
