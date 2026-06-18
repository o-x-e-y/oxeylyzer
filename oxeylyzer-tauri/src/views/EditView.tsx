import { For, Show, createEffect, createMemo, createSignal } from "solid-js";
import { Dof as LibDof } from "libdof";
import KeyboardDisplay from "../components/KeyboardDisplay";
import Dropdown from "../components/Dropdown";
import LayoutSearch from "../components/LayoutSearch";
import { appStore, refreshStore } from "../store";
import { forkLayout, getLayoutDetail, saveLayoutEdit } from "../api";
import type { PhysKey } from "../types";

const FINGER_NAMES = ["LP", "LR", "LM", "LI", "LT", "RT", "RI", "RM", "RR", "RP"] as const;
type FingerName = (typeof FINGER_NAMES)[number];

const FINGER_NAME_TO_IDX: Record<string, number> = Object.fromEntries(
  FINGER_NAMES.map((n, i) => [n, i]),
);

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

function fingerStyle(name: string): string {
  const bg = FINGER_COLORS[name as FingerName] ?? "#666";
  // Perceived luminance — pick text color for best contrast
  const r = parseInt(bg.slice(1, 3), 16);
  const g = parseInt(bg.slice(3, 5), 16);
  const b = parseInt(bg.slice(5, 7), 16);
  const lum = 0.299 * r + 0.587 * g + 0.114 * b;
  const text = lum > 128 ? "#111" : "#eee";
  return `background-color:${bg};color:${text}`;
}

type DofLayer = string[];
type DofJson = {
  name: string;
  board: string;
  layers: Record<string, DofLayer>;
  fingering?: string | string[];
};

type Props = {
  layoutName?: string;
};

export default function EditView(props: Props) {
  const [layoutName, setLayoutName] = createSignal(
    props.layoutName ?? appStore.layouts[0]?.name ?? "",
  );
  const [dof, setDof] = createSignal<DofJson | null>(null);
  const [originalDof, setOriginalDof] = createSignal<DofJson | null>(null);
  const [editedName, setEditedName] = createSignal("");
  const [editedBoard, setEditedBoard] = createSignal("");
  const [editedFingering, setEditedFingering] = createSignal("");
  const [editingIdx, setEditingIdx] = createSignal<number | null>(null);
  const [msg, setMsg] = createSignal<{ text: string; ok: boolean } | null>(null);
  const [loading, setLoading] = createSignal(false);
  const [confirmOverwrite, setConfirmOverwrite] = createSignal(false);
  const [viewMode, setViewMode] = createSignal<"keys" | "fingermap">("keys");
  const [selectedFinger, setSelectedFinger] = createSignal(0);
  const [customFingers, setCustomFingers] = createSignal<number[][] | null>(null);

  createEffect(() => {
    const name = layoutName();
    if (!name) return;
    getLayoutDetail(name)
      .then((raw) => {
        const d = raw as DofJson;
        setDof(d);
        setOriginalDof(d);
        setEditedName(d.name);
        setEditedBoard(d.board);
        setEditedFingering(typeof d.fingering === "string" ? d.fingering : "custom");
        setEditingIdx(null);
        setConfirmOverwrite(false);
        setCustomFingers(null);
        setViewMode("keys");
      })
      .catch((e) => setMsg({ text: String(e), ok: false }));
  });

  const keys = () => {
    const d = dof();
    if (!d) return "";
    const rows = d.layers["main"] ?? [];
    return rows.map((row) => row.replace(/\s+/g, " ").trim().split(/\s+/).join("")).join("");
  };

  const storeLayout = () => appStore.layouts.find((l) => l.name === layoutName());

  // Live keyboard geometry from libdof (updates on board change)
  const liveGeometry = createMemo((): { keyboard: PhysKey[]; shape: number[] } | null => {
    const d = dof();
    if (!d) return null;
    const tryParse = (fingering: string) => {
      try {
        const parsed = new LibDof(JSON.stringify({ ...d, board: editedBoard(), fingering }));
        const board = parsed.board() as { x: number; y: number; width: number; height: number }[][];
        const keyboard: PhysKey[] = board.flat().map((k) => [k.x, k.y, k.width, k.height]);
        const shape: number[] = Array.from(parsed.shape());
        return { keyboard, shape };
      } catch {
        return null;
      }
    };
    return tryParse(editedFingering() || "traditional") ?? tryParse("traditional");
  });

  const currentKeyboard = () => liveGeometry()?.keyboard ?? storeLayout()?.keyboard;
  const currentShape = () => liveGeometry()?.shape ?? storeLayout()?.shape;

  // Per-key finger name strings (flat, aligned with keys()), e.g. "LP", "RI"
  const fingerFlat = createMemo((): string[] | null => {
    if (viewMode() !== "fingermap") return null;
    const d = dof();
    if (!d) return null;
    const cf = customFingers();
    // customFingers stores numeric indices into FINGER_NAMES
    if (cf) return cf.flat().map((f) => FINGER_NAMES[f] as string);
    // Build with the currently-edited board and fingering so named fingerings always resolve
    const tryNamed = (fingering: string): string[] | null => {
      try {
        const parsed = new LibDof(JSON.stringify({ ...d, board: editedBoard(), fingering }));
        return (parsed.fingering() as string[][]).flat();
      } catch {
        return null;
      }
    };
    const primary = editedFingering();
    if (primary && primary !== "custom") {
      return tryNamed(primary) ?? tryNamed("traditional");
    }
    // Layout has explicit row-string fingering stored in d.fingering (string[])
    if (Array.isArray(d.fingering)) {
      return (d.fingering as string[]).flatMap((row) => row.trim().split(/\s+/));
    }
    return tryNamed("traditional");
  });

  const fingerColors = createMemo((): string[] | undefined => {
    const flat = fingerFlat();
    if (!flat) return undefined;
    return flat.map(fingerStyle);
  });

  // Set after a drag completes to suppress the post-drag click event
  let dragJustHappened = false;

  function applyCharAt(rows: string[], pos: number, char: string) {
    let offset = 0;
    for (let r = 0; r < rows.length; r++) {
      const rowChars = rows[r].replace(/\s+/g, " ").trim().split(/\s+/);
      if (pos < offset + rowChars.length) {
        rowChars[pos - offset] = char;
        const left = rowChars.slice(0, 5).join(" ");
        const right = rowChars.slice(5).join(" ");
        rows[r] = rowChars.length <= 5 ? left : `${left}  ${right}`;
        return;
      }
      offset += rowChars.length;
    }
  }

  function updateKeyAt(pos: number, newChar: string) {
    const d = dof();
    if (!d || !newChar) return;
    const rows = [...(d.layers["main"] ?? [])];
    applyCharAt(rows, pos, newChar[0]);
    setDof({ ...d, layers: { ...d.layers, main: rows } });
  }

  function handleSwap(fromIdx: number, toIdx: number) {
    const k = keys();
    const fromChar = k[fromIdx];
    const toChar = k[toIdx];
    if (!fromChar || !toChar || fromIdx === toIdx) return;
    const d = dof();
    if (!d) return;
    const rows = [...(d.layers["main"] ?? [])];
    applyCharAt(rows, fromIdx, toChar);
    applyCharAt(rows, toIdx, fromChar);
    setDof({ ...d, layers: { ...d.layers, main: rows } });
    setEditingIdx(null);
    // Suppress the click that fires on the drag-source element after drop
    dragJustHappened = true;
    setTimeout(() => {
      dragJustHappened = false;
    }, 150);
  }

  function handleEditCommit(idx: number, char: string) {
    if (char) updateKeyAt(idx, char);
  }

  function handleEditNext(idx: number) {
    const total = keys().length;
    if (idx + 1 < total) setEditingIdx(idx + 1);
    else setEditingIdx(null);
  }

  function handleEditBackspace(idx: number) {
    const orig = originalDof();
    if (!orig) return;
    const origKeys = (orig.layers["main"] ?? [])
      .map((row) => row.replace(/\s+/g, " ").trim().split(/\s+/).join(""))
      .join("");
    const origChar = origKeys[idx];
    if (origChar) updateKeyAt(idx, origChar);
  }

  function handleFingerClick(idx: number) {
    const d = dof();
    if (!d) return;
    const shape = currentShape() ?? [];
    let row = -1,
      col = -1;
    let offset = 0;
    for (let r = 0; r < shape.length; r++) {
      if (idx < offset + shape[r]) {
        row = r;
        col = idx - offset;
        break;
      }
      offset += shape[r];
    }
    if (row === -1) return;
    const sf = selectedFinger();
    setCustomFingers((prev) => {
      let base: number[][];
      if (prev) {
        base = prev.map((r) => [...r]);
      } else {
        // Initialise from the current named fingermap so existing assignments are preserved
        try {
          const parsed = new LibDof(
            JSON.stringify({
              ...d,
              board: editedBoard(),
              fingering: editedFingering() || "traditional",
            }),
          );
          const strFingers = parsed.fingering() as string[][];
          base = strFingers.map((r) => r.map((f) => FINGER_NAME_TO_IDX[f] ?? 0));
        } catch {
          // Fallback: parse from raw explicit fingering (string[] of space-separated rows)
          if (Array.isArray(d.fingering)) {
            base = (d.fingering as string[]).map((row) =>
              row
                .trim()
                .split(/\s+/)
                .map((f) => FINGER_NAME_TO_IDX[f] ?? 0),
            );
          } else {
            base = shape.map((len) => Array(len).fill(0));
          }
        }
      }
      if (base[row]) base[row][col] = sf;
      return base;
    });
  }

  function resolveFingeringForSave(): string | string[] {
    const cf = customFingers();
    if (!cf) return editedFingering();
    const d = dof();
    if (!d) return editedFingering();
    // Convert numeric customFingers to string[][] for comparison and serialization
    const cfStrings = cf.map((row) => row.map((f) => FINGER_NAMES[f] as string));
    for (const named of ["angle", "traditional"]) {
      try {
        const test = new LibDof(JSON.stringify({ ...d, board: editedBoard(), fingering: named }));
        const testFingering = test.fingering() as string[][];
        if (
          cfStrings.length === testFingering.length &&
          cfStrings.every(
            (row, r) =>
              row.length === testFingering[r]?.length &&
              row.every((f, c) => f === testFingering[r][c]),
          )
        )
          return named;
      } catch (e) {
        console.error(e);
      }
    }
    return cfStrings.map((row) => row.join(" "));
  }

  async function handleSave() {
    const d = dof();
    if (!d) return;
    const updated: DofJson = {
      ...d,
      name: editedName(),
      board: editedBoard(),
      fingering: resolveFingeringForSave(),
    };

    if (editedName() === layoutName()) {
      if (!confirmOverwrite()) {
        setConfirmOverwrite(true);
        return;
      }
      setLoading(true);
      try {
        await saveLayoutEdit(updated, layoutName());
        await refreshStore();
        setMsg({ text: `Saved "${editedName()}".`, ok: true });
        setConfirmOverwrite(false);
      } catch (e) {
        setMsg({ text: String(e), ok: false });
      } finally {
        setLoading(false);
      }
    } else {
      setLoading(true);
      try {
        await forkLayout(layoutName(), editedName());
        await saveLayoutEdit(updated, editedName());
        await refreshStore();
        setMsg({ text: `Saved as "${editedName()}".`, ok: true });
        setLayoutName(editedName());
      } catch (e) {
        setMsg({ text: String(e), ok: false });
      } finally {
        setLoading(false);
      }
    }
  }

  const fingeringDropdownValue = () => {
    if (customFingers()) return "custom";
    const f = editedFingering();
    return ["traditional", "angle"].includes(f) ? f : "custom";
  };

  return (
    <div class="flex-1 min-h-0 overflow-y-auto flex flex-col gap-4 max-w-3xl">
      <h1 class="text-lg font-mono text-neutral-300">Edit Layout</h1>

      {/* Layout selector */}
      <div class="flex gap-3 items-center">
        <label class="text-sm text-neutral-400 font-mono">Layout</label>
        <LayoutSearch value={layoutName()} onSelect={setLayoutName} />
        <span class="text-sm font-mono text-neutral-400">{layoutName()}</span>
      </div>

      <Show when={dof()}>
        <div class="flex flex-col gap-4">
          {/* Metadata */}
          <div class="border border-neutral-700 p-4 flex flex-col gap-3">
            <div class="text-xs text-neutral-500 uppercase tracking-widest">Metadata</div>
            <div class="grid grid-cols-2 gap-3">
              <div class="flex flex-col gap-1">
                <label class="text-xs text-neutral-500 font-mono">Name</label>
                <div class="flex gap-2 items-center">
                  <input
                    class="bg-neutral-800 border border-neutral-600 text-neutral-100 font-mono text-sm px-2 py-1"
                    value={editedName()}
                    onInput={(e) => {
                      setEditedName(e.currentTarget.value);
                      setConfirmOverwrite(false);
                    }}
                  />
                  <Show when={confirmOverwrite() && editedName() === layoutName()}>
                    <button
                      class="text-xs font-mono text-yellow-400 border border-yellow-700 px-2 py-1 hover:bg-yellow-900/30 whitespace-nowrap"
                      onClick={handleSave}
                    >
                      overwrite?
                    </button>
                  </Show>
                </div>
              </div>
              <div class="flex flex-col gap-1">
                <label class="text-xs text-neutral-500 font-mono">Board</label>
                <Dropdown value={editedBoard()} onChange={setEditedBoard}>
                  {["ortho", "ansi", "iso", "colstag"].map((bt) => (
                    <option value={bt}>{bt}</option>
                  ))}
                </Dropdown>
              </div>
              <div class="flex flex-col gap-1">
                <label class="text-xs text-neutral-500 font-mono">Finger Map</label>
                <div class="flex gap-2 items-center">
                  <Dropdown
                    value={fingeringDropdownValue()}
                    onChange={(v) => {
                      if (v === "traditional" || v === "angle") {
                        setEditedFingering(v);
                        setCustomFingers(null);
                      } else if (["traditional", "angle"].includes(editedFingering())) {
                        setEditedFingering("custom");
                      }
                    }}
                  >
                    <option value="traditional">traditional</option>
                    <option value="angle">angle</option>
                    <option value="custom">custom</option>
                  </Dropdown>
                  <Show when={fingeringDropdownValue() === "custom"}>
                    <input
                      class="bg-neutral-800 border border-neutral-600 text-neutral-100 font-mono text-sm px-2 py-1 w-28"
                      value={
                        ["traditional", "angle"].includes(editedFingering())
                          ? "custom"
                          : editedFingering()
                      }
                      onInput={(e) => setEditedFingering(e.currentTarget.value || "custom")}
                      placeholder="custom"
                    />
                  </Show>
                </div>
              </div>
            </div>
          </div>

          {/* Key editor */}
          <div class="border border-neutral-700 p-4 flex flex-col gap-3">
            <div class="flex items-center justify-between">
              <div class="text-xs text-neutral-500 uppercase tracking-widest">
                <Show when={viewMode() === "keys"} fallback={<span>Finger Map</span>}>
                  Key Layout{" "}
                  <span class="text-neutral-600 normal-case">
                    (click to edit · Tab/Enter advances · Esc cancels)
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
                    keys={keys()}
                    keyboard={kb()}
                    shape={currentShape() ?? []}
                    heatmap={viewMode() === "keys" ? appStore.charFrequencies : undefined}
                    fingerColors={fingerColors()}
                    interactive={true}
                    draggable={true}
                    onSwap={handleSwap}
                    editingIdx={viewMode() === "keys" ? editingIdx() : null}
                    onKeyClick={(_ch, idx) => {
                      if (dragJustHappened) return;
                      if (viewMode() === "keys") setEditingIdx(idx);
                      else handleFingerClick(idx);
                    }}
                    onEditCommit={handleEditCommit}
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

          {/* Save */}
          <div class="flex gap-3 items-center">
            <button
              class="border border-neutral-500 font-mono text-sm px-4 py-1.5 hover:bg-neutral-700 disabled:opacity-40"
              disabled={loading()}
              onClick={handleSave}
            >
              {loading() ? "Saving…" : "Save"}
            </button>
          </div>

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
      </Show>
    </div>
  );
}
