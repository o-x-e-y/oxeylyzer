import { createEffect, createSignal, For, Show } from "solid-js";
import KeyboardDisplay from "../components/KeyboardDisplay";
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
    onNavigateLayouts?: () => void;
};

export default function EditView(props: Props) {
    const [layoutName, setLayoutName] = createSignal(
        props.layoutName ?? appStore.layouts[0]?.name ?? "",
    );
    const [dof, setDof] = createSignal<DofJson | null>(null);
    const [editedName, setEditedName] = createSignal("");
    const [editedBoard, setEditedBoard] = createSignal("");
    const [editedFingering, setEditedFingering] = createSignal("");
    const [selectedKey, setSelectedKey] = createSignal<number | null>(null);
    const [keyInput, setKeyInput] = createSignal("");
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
            })
            .catch((e) => setMsg({ text: String(e), ok: false }));
    });

    /** Reconstructs the 30-char key string from the Dof main layer rows. */
    const keys = () => {
        const d = dof();
        if (!d) return "";
        const rows = d.layers["main"] ?? [];
        return rows
            .map((row) => row.replace(/\s+/g, " ").trim().split(/\s+/).join(""))
            .join("");
    };

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
                // Preserve the 5|5 split spacing
                const left = rowChars.slice(0, 5).join(" ");
                const right = rowChars.slice(5).join(" ");
                rows[r] = rowChars.length <= 5 ? left : `${left}  ${right}`;
                break;
            }
            offset += rowLen;
        }
        setDof({ ...d, layers: { ...d.layers, main: rows } });
        setSelectedKey(null);
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
                <select
                    class="bg-neutral-800 border border-neutral-600 text-neutral-100 font-mono text-sm px-2 py-1"
                    value={layoutName()}
                    onChange={(e) => setLayoutName(e.currentTarget.value)}
                >
                    {appStore.layouts.map((l) => (
                        <option value={l.name}>{l.name}</option>
                    ))}
                </select>
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
                                    class="bg-neutral-800 border border-neutral-600 text-neutral-100 font-mono text-sm px-2 py-1"
                                    value={editedBoard()}
                                    onChange={(e) => setEditedBoard(e.currentTarget.value)}
                                >
                                    <For each={["ortho", "ansi", "iso", "colstag", "rowstag"]}>
                                        {(bt) => <option value={bt}>{bt}</option>}
                                    </For>
                                </select>
                            </div>
                            <div class="flex flex-col gap-1">
                                <label class="text-xs text-neutral-500 font-mono">
                                    Fingering
                                </label>
                                <input
                                    class="bg-neutral-800 border border-neutral-600 text-neutral-100 font-mono text-sm px-2 py-1"
                                    value={editedFingering()}
                                    onInput={(e) => setEditedFingering(e.currentTarget.value)}
                                />
                            </div>
                        </div>
                    </div>

                    {/* Key editor — click a key to remap it */}
                    <div class="border border-neutral-700 p-4 flex flex-col gap-3">
                        <div class="text-xs text-neutral-500 uppercase tracking-widest">
                            Key Layout{" "}
                            <span class="text-neutral-600 normal-case">
                                (click a key to change it)
                            </span>
                        </div>
                        <Show when={keys().length >= 30}>
                            <KeyboardDisplay
                                keys={keys()}
                                interactive={true}
                                heatmap={appStore.charFrequencies}
                                onKeyClick={(char) => {
                                    const pos = keys().indexOf(char);
                                    setSelectedKey(pos >= 0 ? pos : null);
                                    setKeyInput(char);
                                }}
                            />
                        </Show>
                        <Show when={selectedKey() !== null}>
                            <div class="flex items-center gap-3 text-sm font-mono">
                                <span class="text-neutral-400">
                                    Replace key #{selectedKey()! + 1} (
                                    <span class="text-neutral-200">{keys()[selectedKey()!]}</span>
                                    ) with:
                                </span>
                                <input
                                    class="bg-neutral-800 border border-neutral-600 text-neutral-100 font-mono text-sm px-2 py-1 w-12 text-center"
                                    maxLength={1}
                                    value={keyInput()}
                                    onInput={(e) => setKeyInput(e.currentTarget.value)}
                                    onKeyDown={(e) => {
                                        if (e.key === "Enter") updateKeyAt(selectedKey()!, keyInput());
                                        if (e.key === "Escape") setSelectedKey(null);
                                    }}
                                    ref={(el) => setTimeout(() => el?.focus(), 0)}
                                />
                                <button
                                    class="border border-neutral-500 px-2 py-0.5 hover:bg-neutral-700"
                                    onClick={() => updateKeyAt(selectedKey()!, keyInput())}
                                >
                                    Set
                                </button>
                                <button
                                    class="text-neutral-500 hover:text-neutral-300"
                                    onClick={() => setSelectedKey(null)}
                                >
                                    ×
                                </button>
                            </div>
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
