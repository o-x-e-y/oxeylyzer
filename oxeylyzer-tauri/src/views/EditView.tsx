import { createEffect, createSignal, Show } from "solid-js";
import KeyboardDisplay from "../components/KeyboardDisplay";
import LayoutSearch from "../components/LayoutSearch";
import { appStore, initStore } from "../store";
import { getLayoutDetail, saveLayoutEdit, forkLayout } from "../api";

type DofLayer = string[];
type DofJson = {
    name: string;
    board: string;
    layers: Record<string, DofLayer>;
    fingering: string;
};

type Props = {
    layoutName?: string;
};

export default function EditView(props: Props) {
    const [layoutName, setLayoutName] = createSignal(
        props.layoutName ?? appStore.layouts[0]?.name ?? "",
    );
    const [dof, setDof] = createSignal<DofJson | null>(null);
    const [editedName, setEditedName] = createSignal("");
    const [editedBoard, setEditedBoard] = createSignal("");
    const [editedFingering, setEditedFingering] = createSignal("");
    const [editingIdx, setEditingIdx] = createSignal<number | null>(null);
    const [msg, setMsg] = createSignal<{ text: string; ok: boolean } | null>(null);
    const [forkName, setForkName] = createSignal("");
    const [loading, setLoading] = createSignal(false);

    createEffect(() => {
        const name = layoutName();
        if (!name) return;
        getLayoutDetail(name)
            .then((raw) => {
                const d = raw as DofJson;
                setDof(d);
                setEditedName(d.name);
                setEditedBoard(d.board);
                setEditedFingering(d.fingering);
                setEditingIdx(null);
            })
            .catch((e) => setMsg({ text: String(e), ok: false }));
    });

    const keys = () => {
        const d = dof();
        if (!d) return "";
        const rows = d.layers["main"] ?? [];
        return rows
            .map((row) => row.replace(/\s+/g, " ").trim().split(/\s+/).join(""))
            .join("");
    };

    // Use keyboard/shape from the store if available for the current layout
    const storeLayout = () => appStore.layouts.find((l) => l.name === layoutName());

    function updateKeyAt(pos: number, newChar: string) {
        const d = dof();
        if (!d || !newChar) return;
        const c = newChar[0];
        const rows = [...(d.layers["main"] ?? [])];
        let offset = 0;
        for (let r = 0; r < rows.length; r++) {
            const rowChars = rows[r].replace(/\s+/g, " ").trim().split(/\s+/);
            const rowLen = rowChars.length;
            if (pos < offset + rowLen) {
                const inRow = pos - offset;
                rowChars[inRow] = c;
                const left = rowChars.slice(0, 5).join(" ");
                const right = rowChars.slice(5).join(" ");
                rows[r] = rowChars.length <= 5 ? left : `${left}  ${right}`;
                break;
            }
            offset += rowLen;
        }
        setDof({ ...d, layers: { ...d.layers, main: rows } });
    }

    function handleEditCommit(idx: number, char: string) {
        // Only update the key; Enter/Tab (onEditNext) is what advances.
        if (char) updateKeyAt(idx, char);
    }

    function handleEditNext(idx: number) {
        const total = keys().length;
        if (idx + 1 < total) setEditingIdx(idx + 1);
        else setEditingIdx(null);
    }

    async function handleSave() {
        if (!dof()) return;
        const updated: DofJson = {
            ...dof()!,
            name: editedName(),
            board: editedBoard(),
            fingering: editedFingering(),
        };
        setLoading(true);
        try {
            await saveLayoutEdit(updated, layoutName());
            await initStore();
            setMsg({ text: `Saved "${editedName()}".`, ok: true });
            setLayoutName(editedName());
        } catch (e) {
            setMsg({ text: String(e), ok: false });
        } finally {
            setLoading(false);
        }
    }

    async function handleFork() {
        const name = forkName().trim();
        if (!name) return;
        setLoading(true);
        try {
            await forkLayout(layoutName(), name);
            await initStore();
            setMsg({ text: `Forked as "${name}".`, ok: true });
            setForkName("");
            setLayoutName(name);
        } catch (e) {
            setMsg({ text: String(e), ok: false });
        } finally {
            setLoading(false);
        }
    }

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
                    {/* Metadata form */}
                    <div class="border border-neutral-700 p-4 flex flex-col gap-3">
                        <div class="text-xs text-neutral-500 uppercase tracking-widest">
                            Metadata
                        </div>
                        <div class="grid grid-cols-2 gap-3">
                            <div class="flex flex-col gap-1">
                                <label class="text-xs text-neutral-500 font-mono">Name</label>
                                <input
                                    class="bg-neutral-800 border border-neutral-600 text-neutral-100 font-mono text-sm px-2 py-1"
                                    value={editedName()}
                                    onInput={(e) => setEditedName(e.currentTarget.value)}
                                />
                            </div>
                            <div class="flex flex-col gap-1">
                                <label class="text-xs text-neutral-500 font-mono">Board</label>
                                <select
                                    class="appearance-none bg-neutral-800 border border-neutral-600 text-neutral-100 font-mono text-sm px-2 py-1"
                                    value={editedBoard()}
                                    onChange={(e) => setEditedBoard(e.currentTarget.value)}
                                >
                                    {["ortho", "ansi", "iso", "colstag", "rowstag"].map((bt) => (
                                        <option value={bt}>{bt}</option>
                                    ))}
                                </select>
                            </div>
                            <div class="flex flex-col gap-1">
                                <label class="text-xs text-neutral-500 font-mono">Fingering</label>
                                <input
                                    class="bg-neutral-800 border border-neutral-600 text-neutral-100 font-mono text-sm px-2 py-1"
                                    value={editedFingering()}
                                    onInput={(e) => setEditedFingering(e.currentTarget.value)}
                                />
                            </div>
                        </div>
                    </div>

                    {/* Key editor — click to edit inline */}
                    <div class="border border-neutral-700 p-4 flex flex-col gap-3">
                        <div class="text-xs text-neutral-500 uppercase tracking-widest">
                            Key Layout{" "}
                            <span class="text-neutral-600 normal-case">
                                (click a key to edit · Tab/Enter advances · Esc cancels)
                            </span>
                        </div>
                        <Show when={keys().length >= 30 && storeLayout()}>
                            {(bl) => (
                                <KeyboardDisplay
                                    keys={keys()}
                                    keyboard={bl().keyboard}
                                    shape={bl().shape}
                                    heatmap={appStore.charFrequencies}
                                    interactive={true}
                                    editingIdx={editingIdx()}
                                    onKeyClick={(_ch, idx) => setEditingIdx(idx)}
                                    onEditCommit={handleEditCommit}
                                    onEditNext={handleEditNext}
                                    onEditCancel={() => setEditingIdx(null)}
                                    class="max-w-sm"
                                />
                            )}
                        </Show>
                    </div>

                    {/* Save / Fork */}
                    <div class="flex gap-4 items-start">
                        <button
                            class="border border-neutral-500 font-mono text-sm px-4 py-1.5 hover:bg-neutral-700 disabled:opacity-40"
                            disabled={loading()}
                            onClick={handleSave}
                        >
                            {loading() ? "Saving…" : "Save"}
                        </button>

                        <div class="flex flex-col gap-1">
                            <div class="flex gap-2">
                                <input
                                    class="bg-neutral-800 border border-neutral-600 text-neutral-100 font-mono text-sm px-2 py-1 w-44"
                                    placeholder="fork as name…"
                                    value={forkName()}
                                    onInput={(e) => setForkName(e.currentTarget.value)}
                                    onKeyDown={(e) => e.key === "Enter" && handleFork()}
                                />
                                <button
                                    class="border border-neutral-600 font-mono text-sm px-3 py-1 hover:bg-neutral-700 disabled:opacity-40"
                                    disabled={loading() || !forkName().trim()}
                                    onClick={handleFork}
                                >
                                    Fork
                                </button>
                            </div>
                            <div class="text-xs text-neutral-600 font-mono">
                                Creates a copy under a new name
                            </div>
                        </div>
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
