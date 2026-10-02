import { For, Show, createEffect, createMemo, createSignal, on, untrack } from "solid-js";
import { Dof as LibDof } from "libdof";
import KeyboardDisplay from "../components/KeyboardDisplay";
import Dropdown from "../components/Dropdown";
import LayoutSearch from "../components/LayoutSearch";
import { appStore, refreshStore } from "../store";
import { getLayoutDetail, saveLayoutEdit } from "../api";
import type { PhysKey } from "../types";
import { charToken, flatTokens, setToken, tokenLabel } from "../dof";

const FINGER_NAMES = ["LP", "LR", "LM", "LI", "LT", "RT", "RI", "RM", "RR", "RP"] as const;
type FingerName = (typeof FINGER_NAMES)[number];

const FINGER_COLORS: Record<FingerName, string> = {
  LP: "#ffcdd2",
  LR: "#f87680",
  LM: "#e92832",
  LI: "#9a191c",
  LT: "#531313",
  RT: "#09243d",
  RI: "#125490",
  RM: "#1786e7",
  RR: "#67b3f3",
  RP: "#BBDEFB",
};

const BOARDS = ["ansi", "iso", "ortho", "colstag"];
const NAMED_FINGERINGS = ["traditional", "angle"];

function fingerStyle(name: string): string {
  const bg = FINGER_COLORS[name.toUpperCase() as FingerName] ?? "#666";
  // Perceived luminance — pick text color for best contrast
  const r = parseInt(bg.slice(1, 3), 16);
  const g = parseInt(bg.slice(3, 5), 16);
  const b = parseInt(bg.slice(5, 7), 16);
  const lum = 0.299 * r + 0.587 * g + 0.114 * b;
  const text = lum > 128 ? "#111" : "#eee";
  return `background-color:${bg};color:${text}`;
}

/** A .dof file as JSON. Fields the editor doesn't touch are written back unchanged. */
type DofJson = Record<string, unknown> & {
  name: string;
  /** A board name, or explicit key geometry. */
  board: unknown;
  layers: Record<string, string[]>;
  /** A named fingering, or explicit rows of finger names. */
  fingering?: string | string[];
};

type Parsed = { keyboard: PhysKey[]; shape: number[]; fingers: string[] };

function parseDof(d: DofJson): Parsed | { error: string } {
  let parsed: LibDof | undefined;
  try {
    parsed = new LibDof(JSON.stringify(d));
    const board = parsed.board() as { x: number; y: number; width: number; height: number }[][];
    return {
      keyboard: board.flat().map((k) => [k.x, k.y, k.width, k.height]),
      shape: Array.from(parsed.shape()),
      fingers: (parsed.fingering() as string[][]).flat(),
    };
  } catch (e) {
    return { error: String(e) };
  } finally {
    parsed?.free();
  }
}

const isParsed = (p: Parsed | { error: string } | null): p is Parsed => !!p && !("error" in p);

type Props = {
  layoutName?: string;
};

export default function EditView(props: Props) {
  const [layoutName, setLayoutName] = createSignal("");
  const [original, setOriginal] = createSignal<DofJson | null>(null);
  const [dof, setDof] = createSignal<DofJson | null>(null);
  const [editingIdx, setEditingIdx] = createSignal<number | null>(null);
  const [msg, setMsg] = createSignal<{ text: string; ok: boolean } | null>(null);
  const [saving, setSaving] = createSignal(false);
  const [confirmOverwrite, setConfirmOverwrite] = createSignal(false);
  const [viewMode, setViewMode] = createSignal<"keys" | "fingermap">("keys");
  const [selectedFinger, setSelectedFinger] = createSignal(0);

  let loadSeq = 0;
  async function load(name: string) {
    const s = ++loadSeq;
    try {
      const d = (await getLayoutDetail(name)) as DofJson;
      if (s !== loadSeq) return;
      setLayoutName(name);
      setDof(d);
      setOriginal(d);
      setEditingIdx(null);
      setConfirmOverwrite(false);
      setViewMode("keys");
      setMsg(null);
    } catch (e) {
      if (s === loadSeq) setMsg({ text: String(e), ok: false });
    }
  }

  createEffect(
    on(
      () => props.layoutName,
      (layoutName) => {
        const name = layoutName ?? untrack(() => appStore.layouts[0]?.name);
        if (name) load(name);
      },
    ),
  );

  const update = (changes: Partial<DofJson>) => {
    const d = dof();
    if (d) setDof({ ...d, ...changes });
  };

  const mainRows = () => dof()?.layers?.main ?? [];
  const tokens = () => flatTokens(mainRows());
  const labels = () => tokens().map(tokenLabel);
  const dirty = () => JSON.stringify(dof()) !== JSON.stringify(original());

  const parsed = createMemo(() => {
    const d = dof();
    return d ? parseDof(d) : null;
  });
  const parseError = () => {
    const p = parsed();
    return p && "error" in p ? p.error : null;
  };
  const okParsed = () => {
    const p = parsed();
    return isParsed(p) ? p : null;
  };

  // While the edit doesn't parse, keep showing the saved layout's geometry.
  const storeLayout = () =>
    appStore.layouts.find((l) => l.name.toLowerCase() === layoutName().toLowerCase());
  const currentKeyboard = () => okParsed()?.keyboard ?? storeLayout()?.keyboard;
  const currentShape = () => okParsed()?.shape ?? storeLayout()?.shape ?? [];

  const fingeringMode = () => {
    const f = dof()?.fingering;
    if (Array.isArray(f)) return "custom";
    const name = (f ?? "traditional").toLowerCase();
    return name === "standard" ? "traditional" : name;
  };
  // Named fingerings only exist for some boards (no angle on ortho/colstag).
  const fingeringValid = createMemo(() => {
    const d = dof();
    const valid: Record<string, boolean> = {};
    for (const name of NAMED_FINGERINGS) valid[name] = !!d && isParsed(parseDof({ ...d, fingering: name }));
    return valid;
  });

  /** The current fingering as explicit rows, ready to paint key by key. */
  const explicitFingering = (): string[] => {
    const d = dof();
    if (Array.isArray(d?.fingering)) return d.fingering;
    const fingers = okParsed()?.fingers ?? [];
    let offset = 0;
    return currentShape().map((n) => {
      const row = fingers.slice(offset, offset + n).join(" ");
      offset += n;
      return row;
    });
  };

  function setFingering(mode: string) {
    update({ fingering: mode === "custom" ? explicitFingering() : mode });
  }

  function paintFinger(idx: number) {
    update({ fingering: setToken(explicitFingering(), idx, FINGER_NAMES[selectedFinger()]) });
  }

  /** Explicit rows that match a named fingering are saved as that name. */
  function normalizedFingering(d: DofJson): DofJson["fingering"] {
    if (!Array.isArray(d.fingering)) return d.fingering;
    const mine = flatTokens(d.fingering)
      .map((f) => f.toUpperCase())
      .join(" ");
    for (const name of NAMED_FINGERINGS) {
      const p = parseDof({ ...d, fingering: name });
      if (isParsed(p) && p.fingers.join(" ") === mine) return name;
    }
    return d.fingering;
  }

  const fingerColors = () =>
    viewMode() === "fingermap" ? okParsed()?.fingers.map(fingerStyle) : undefined;

  const setMainToken = (idx: number, token: string) =>
    update({ layers: { ...dof()!.layers, main: setToken(mainRows(), idx, token) } });

  // Set after a drag completes to suppress the post-drag click event
  let dragJustHappened = false;

  function handleSwap(fromIdx: number, toIdx: number) {
    const t = tokens();
    if (fromIdx === toIdx || !t[fromIdx] || !t[toIdx]) return;
    const swapped = setToken(setToken(mainRows(), fromIdx, t[toIdx]), toIdx, t[fromIdx]);
    update({ layers: { ...dof()!.layers, main: swapped } });
    setEditingIdx(null);
    // Suppress the click that fires on the drag-source element after drop
    dragJustHappened = true;
    setTimeout(() => {
      dragJustHappened = false;
    }, 150);
  }

  function handleEditNext(idx: number) {
    setEditingIdx(idx + 1 < tokens().length ? idx + 1 : null);
  }

  function handleEditBackspace(idx: number) {
    const token = flatTokens(original()?.layers?.main ?? [])[idx];
    if (token) setMainToken(idx, token);
  }

  const trimmedName = () => dof()?.name.trim() ?? "";
  const overwriting = () => trimmedName().toLowerCase() === layoutName().toLowerCase();

  async function handleSave() {
    const d = dof();
    const name = trimmedName();
    if (!d || !name || parseError()) return;
    const overwrite = overwriting();
    if (overwrite && !confirmOverwrite()) {
      setConfirmOverwrite(true);
      return;
    }
    setSaving(true);
    try {
      await saveLayoutEdit({ ...d, name, fingering: normalizedFingering(d) }, layoutName());
      await refreshStore();
      await load(name);
      setMsg({ text: overwrite ? `Saved "${name}".` : `Saved as new layout "${name}".`, ok: true });
    } catch (e) {
      setMsg({ text: String(e), ok: false });
    } finally {
      setSaving(false);
      setConfirmOverwrite(false);
    }
  }

  const saveLabel = () => {
    if (saving()) return "Saving…";
    if (!overwriting()) return "Save as new layout";
    return confirmOverwrite() ? "Click again to overwrite" : "Save";
  };

  return (
    <div class="flex-1 min-h-0 overflow-y-auto flex flex-col gap-4 max-w-3xl">
      <h1 class="text-lg font-mono text-neutral-300">Edit Layout</h1>

      {/* Layout selector */}
      <div class="flex gap-3 items-center">
        <label class="text-sm text-neutral-400 font-mono">Layout</label>
        <LayoutSearch value={layoutName()} onSelect={load} />
        <span class="text-sm font-mono text-neutral-400">{layoutName()}</span>
        <Show when={dirty()}>
          <span class="text-xs font-mono text-yellow-500">unsaved changes</span>
        </Show>
      </div>

      <Show when={dof()}>
        {(d) => (
          <div class="flex flex-col gap-4">
            {/* Metadata */}
            <div class="border border-neutral-700 p-4 flex flex-col gap-3">
              <div class="text-xs text-neutral-500 uppercase tracking-widest">Metadata</div>
              <div class="grid grid-cols-2 gap-3">
                <div class="flex flex-col gap-1">
                  <label class="text-xs text-neutral-500 font-mono">Name</label>
                  <input
                    class="bg-neutral-800 border border-neutral-600 text-neutral-100 font-mono text-sm px-2 py-1"
                    value={d().name}
                    onInput={(e) => {
                      update({ name: e.currentTarget.value });
                      setConfirmOverwrite(false);
                    }}
                  />
                </div>
                <div class="flex flex-col gap-1">
                  <label class="text-xs text-neutral-500 font-mono">Board</label>
                  <Show
                    when={typeof d().board === "string"}
                    fallback={
                      <span class="text-sm font-mono text-neutral-400 py-1">
                        custom key geometry (kept as is)
                      </span>
                    }
                  >
                    <Dropdown value={d().board as string} onChange={(board) => update({ board })}>
                      <For each={BOARDS.includes(d().board as string) ? BOARDS : [d().board as string, ...BOARDS]}>
                        {(bt) => <option value={bt}>{bt}</option>}
                      </For>
                    </Dropdown>
                  </Show>
                </div>
                <div class="flex flex-col gap-1">
                  <label class="text-xs text-neutral-500 font-mono">Finger Map</label>
                  <Dropdown value={fingeringMode()} onChange={setFingering}>
                    <For each={NAMED_FINGERINGS}>
                      {(name) => (
                        <option value={name} disabled={!fingeringValid()[name]}>
                          {name}
                          {fingeringValid()[name] ? "" : " (not for this board)"}
                        </option>
                      )}
                    </For>
                    <option value="custom">custom</option>
                  </Dropdown>
                </div>
              </div>
            </div>

            {/* Key editor */}
            <div class="border border-neutral-700 p-4 flex flex-col gap-3">
              <div class="flex items-center justify-between">
                <div class="text-xs text-neutral-500 uppercase tracking-widest">
                  <Show
                    when={viewMode() === "keys"}
                    fallback={
                      <>
                        Finger Map{" "}
                        <span class="text-neutral-600 normal-case">
                          (pick a finger, then click keys)
                        </span>
                      </>
                    }
                  >
                    Key Layout{" "}
                    <span class="text-neutral-600 normal-case">
                      (click to edit · Tab/Enter advances · Backspace restores · Esc cancels)
                    </span>
                  </Show>
                </div>
                <div class="flex gap-1">
                  <button
                    class="font-mono text-xs px-2 py-0.5 border"
                    classList={{
                      "border-neutral-400 text-neutral-200": viewMode() === "keys",
                      "border-neutral-700 text-neutral-500 hover:border-neutral-500":
                        viewMode() !== "keys",
                    }}
                    onClick={() => setViewMode("keys")}
                  >
                    Keys
                  </button>
                  <button
                    class="font-mono text-xs px-2 py-0.5 border"
                    classList={{
                      "border-neutral-400 text-neutral-200": viewMode() === "fingermap",
                      "border-neutral-700 text-neutral-500 hover:border-neutral-500":
                        viewMode() !== "fingermap",
                    }}
                    onClick={() => setViewMode("fingermap")}
                  >
                    Finger Map
                  </button>
                </div>
              </div>

              <Show when={currentKeyboard()}>
                {(kb) => (
                  <>
                    <KeyboardDisplay
                      keys={labels()}
                      keyboard={kb()}
                      shape={currentShape()}
                      heatmap={viewMode() === "keys" ? appStore.charFrequencies : undefined}
                      fingerColors={fingerColors()}
                      interactive={true}
                      draggable={viewMode() === "keys"}
                      onSwap={handleSwap}
                      editingIdx={viewMode() === "keys" ? editingIdx() : null}
                      onKeyClick={(_ch, idx) => {
                        if (dragJustHappened) return;
                        if (viewMode() === "keys") setEditingIdx(idx);
                        else paintFinger(idx);
                      }}
                      onEditCommit={(idx, ch) => setMainToken(idx, charToken(ch))}
                      onEditNext={handleEditNext}
                      onEditCancel={() => setEditingIdx(null)}
                      onEditBackspace={handleEditBackspace}
                      class="max-w-sm"
                    />
                    <Show when={viewMode() === "fingermap"}>
                      <div class="flex gap-0.5 max-w-sm">
                        <For each={FINGER_NAMES as unknown as FingerName[]}>
                          {(name, i) => (
                            <button
                              class="flex-1 h-6 text-[10px] font-mono border leading-none"
                              style={`${fingerStyle(name)};border-color:${selectedFinger() === i() ? "#fff" : "transparent"}`}
                              onClick={() => setSelectedFinger(i())}
                            >
                              {name}
                            </button>
                          )}
                        </For>
                      </div>
                    </Show>
                  </>
                )}
              </Show>
            </div>

            <Show when={parseError()}>
              <div class="text-xs font-mono border border-red-800 text-red-400 px-2 py-1">
                {parseError()}
              </div>
            </Show>

            {/* Save */}
            <div class="flex gap-3 items-center">
              <button
                class="border font-mono text-sm px-4 py-1.5 disabled:opacity-40"
                classList={{
                  "border-yellow-700 text-yellow-400 hover:bg-yellow-900/30": confirmOverwrite(),
                  "border-neutral-500 hover:bg-neutral-700": !confirmOverwrite(),
                }}
                disabled={saving() || !trimmedName() || !!parseError()}
                onClick={handleSave}
              >
                {saveLabel()}
              </button>
              <Show when={!overwriting() && trimmedName()}>
                <span class="text-xs font-mono text-neutral-500">
                  renamed — "{layoutName()}" stays as it is
                </span>
              </Show>
            </div>
          </div>
        )}
      </Show>

      <Show when={msg()}>
        {(m) => (
          <div
            class="text-xs font-mono border px-2 py-1"
            classList={{
              "border-neutral-700 text-neutral-400": m().ok,
              "border-red-800 text-red-400": !m().ok,
            }}
          >
            {m().text}
          </div>
        )}
      </Show>
    </div>
  );
}
