import { createSignal, For, onMount, Show } from "solid-js";
import Dropdown from "../components/Dropdown";
import {
    getConfig, setConfig, getDefaults,
    listWeightPresets, saveWeightPreset, loadWeightPreset,
    type ConfigDto, type WeightsDto,
} from "../api";
import { initStore } from "../store";

function NumInput(props: {
    label: string;
    value: number;
    onChange: (v: number) => void;
    step?: number;
    tooltip?: string;
}) {
    return (
        <div class="flex items-center gap-2">
            <label
                class="text-xs text-neutral-400 font-mono w-44 shrink-0"
                title={props.tooltip}
            >
                {props.label}
            </label>
            <input
                type="number"
                class="bg-neutral-800 border border-neutral-600 text-neutral-100 font-mono text-sm px-2 py-0.5 w-28 text-right"
                value={props.value}
                step={props.step ?? 0.1}
                onInput={(e) => {
                    const v = parseFloat(e.currentTarget.value);
                    if (!isNaN(v)) props.onChange(v);
                }}
            />
        </div>
    );
}

export default function ConfigView() {
    const [config, setConfigState] = createSignal<ConfigDto | null>(null);
    const [saving, setSaving] = createSignal(false);
    const [msg, setMsg] = createSignal<{ text: string; ok: boolean } | null>(null);
    const [presetName, setPresetName] = createSignal("");
    const [presets, setPresets] = createSignal<string[]>([]);

    onMount(async () => {
        try {
            const [cfg, ps] = await Promise.all([getConfig(), listWeightPresets()]);
            setConfigState(cfg);
            setPresets(ps);
        } catch (e) {
            setMsg({ text: String(e), ok: false });
        }
    });

    function updateWeight(key: keyof WeightsDto, value: number) {
        const c = config();
        if (!c) return;
        setConfigState({ ...c, weights: { ...c.weights, [key]: value } });
    }

    function updateFingerWeight(key: string, value: number) {
        const c = config();
        if (!c) return;
        setConfigState({
            ...c,
            weights: {
                ...c.weights,
                finger_weights: { ...c.weights.finger_weights, [key]: value },
            },
        });
    }

    function updateMaxFingerUse(key: string, value: number) {
        const c = config();
        if (!c) return;
        setConfigState({
            ...c,
            weights: {
                ...c.weights,
                max_finger_use: { ...c.weights.max_finger_use, [key]: value },
            },
        });
    }

    async function handleSave() {
        const c = config();
        if (!c) return;
        setSaving(true);
        setMsg(null);
        try {
            await setConfig(c);
            await initStore();
            setMsg({ text: "Config saved and engine reloaded.", ok: true });
        } catch (e) {
            setMsg({ text: String(e), ok: false });
        } finally {
            setSaving(false);
        }
    }

    async function handleResetDefaults() {
        try {
            const defaults = await getDefaults();
            setConfigState((c) => c ? { ...c, weights: defaults.weights } : null);
        } catch (e) {
            setMsg({ text: String(e), ok: false });
        }
    }

    async function savePreset() {
        const name = presetName().trim();
        const c = config();
        if (!name || !c) return;
        try {
            await saveWeightPreset(name, c.weights);
            const ps = await listWeightPresets();
            setPresets(ps);
            setPresetName("");
            setMsg({ text: `Preset "${name}" saved.`, ok: true });
        } catch (e) {
            setMsg({ text: String(e), ok: false });
        }
    }

    async function handleLoadPreset(name: string) {
        const c = config();
        if (!c || !name) return;
        try {
            const weights = await loadWeightPreset(name);
            setConfigState({ ...c, weights });
        } catch (e) {
            setMsg({ text: String(e), ok: false });
        }
    }

    function exportWeights() {
        const c = config();
        if (!c) return;
        const w = c.weights;
        const fw = w.finger_weights;
        const mfu = w.max_finger_use;
        const text = `[weights]
lateral_penalty = ${w.lateral_penalty}
sfbs = ${w.sfbs}
sfs = ${w.sfs}
stretches = ${w.stretches}
pinky_ring_bigrams = ${w.pinky_ring_bigrams}
inrolls = ${w.inrolls}
outrolls = ${w.outrolls}
onehands = ${w.onehands}
alternates = ${w.alternates}
alternates_sfs = ${w.alternates_sfs}
redirects = ${w.redirects}
redirects_sfs = ${w.redirects_sfs}
bad_redirects = ${w.bad_redirects}
bad_redirects_sfs = ${w.bad_redirects_sfs}

[weights.finger_weights]
lp=${fw.lp} lr=${fw.lr} lm=${fw.lm} li=${fw.li} lt=${fw.lt}
rt=${fw.rt} ri=${fw.ri} rm=${fw.rm} rr=${fw.rr} rp=${fw.rp}

[weights.max_finger_use]
penalty=${mfu.penalty} pinky=${mfu.pinky} ring=${mfu.ring}
middle=${mfu.middle} index=${mfu.index} thumb=${mfu.thumb}`;
        navigator.clipboard.writeText(text).then(() =>
            setMsg({ text: "Weights copied to clipboard.", ok: true }),
        );
    }

    return (
        <div class="flex-1 min-h-0 overflow-y-auto flex flex-col gap-4 max-w-2xl">
            <h1 class="text-lg font-mono text-neutral-300">Config</h1>

            <Show when={config()}>
                {(cfg) => (
                    <div class="flex flex-col gap-4">
                        {/* Paths & limits */}
                        <section class="border border-neutral-700 p-4 flex flex-col gap-3">
                            <div class="text-xs text-neutral-500 uppercase tracking-widest">
                                Paths & Limits
                            </div>
                            <div class="flex flex-col gap-1">
                                <label class="text-xs text-neutral-500 font-mono">
                                    Corpus path
                                </label>
                                <input
                                    class="bg-neutral-800 border border-neutral-600 text-neutral-100 font-mono text-xs px-2 py-1"
                                    value={cfg().corpus}
                                    onInput={(e) =>
                                        setConfigState({ ...cfg(), corpus: e.currentTarget.value })
                                    }
                                />
                            </div>
                            <div class="grid grid-cols-2 gap-3">
                                <NumInput
                                    label="Trigram precision"
                                    value={cfg().trigram_precision}
                                    step={1000}
                                    tooltip="Number of trigrams used for scoring. Higher = more accurate but slower."
                                    onChange={(v) =>
                                        setConfigState({
                                            ...cfg(),
                                            trigram_precision: Math.floor(v),
                                        })
                                    }
                                />
                                <NumInput
                                    label="Max cores"
                                    value={cfg().max_cores}
                                    step={1}
                                    tooltip="Maximum CPU threads used for generation."
                                    onChange={(v) =>
                                        setConfigState({ ...cfg(), max_cores: Math.floor(v) })
                                    }
                                />
                            </div>
                        </section>

                        {/* Bigram penalties */}
                        <section class="border border-neutral-700 p-4 flex flex-col gap-2">
                            <div class="text-xs text-neutral-500 uppercase tracking-widest mb-1">
                                Bigram Weights
                            </div>
                            <NumInput
                                label="sfbs"
                                value={cfg().weights.sfbs}
                                tooltip="Same-finger bigram penalty (negative = penalty)"
                                onChange={(v) => updateWeight("sfbs", v)}
                            />
                            <NumInput
                                label="sfs (skip)"
                                value={cfg().weights.sfs}
                                tooltip="Same-finger skip penalty"
                                onChange={(v) => updateWeight("sfs", v)}
                            />
                            <NumInput
                                label="lateral_penalty"
                                value={cfg().weights.lateral_penalty}
                                tooltip="Multiplier applied to lateral stretch distances"
                                onChange={(v) => updateWeight("lateral_penalty", v)}
                            />
                            <NumInput
                                label="stretches"
                                value={cfg().weights.stretches}
                                tooltip="Stretch bigram penalty"
                                onChange={(v) => updateWeight("stretches", v)}
                            />
                            <NumInput
                                label="pinky_ring_bigrams"
                                value={cfg().weights.pinky_ring_bigrams}
                                tooltip="Pinky-ring bigram penalty"
                                onChange={(v) => updateWeight("pinky_ring_bigrams", v)}
                            />
                        </section>

                        {/* Trigram rewards/penalties */}
                        <section class="border border-neutral-700 p-4 flex flex-col gap-2">
                            <div class="text-xs text-neutral-500 uppercase tracking-widest mb-1">
                                Trigram Weights
                            </div>
                            <NumInput
                                label="inrolls"
                                value={cfg().weights.inrolls}
                                onChange={(v) => updateWeight("inrolls", v)}
                            />
                            <NumInput
                                label="outrolls"
                                value={cfg().weights.outrolls}
                                onChange={(v) => updateWeight("outrolls", v)}
                            />
                            <NumInput
                                label="onehands"
                                value={cfg().weights.onehands}
                                onChange={(v) => updateWeight("onehands", v)}
                            />
                            <NumInput
                                label="alternates"
                                value={cfg().weights.alternates}
                                onChange={(v) => updateWeight("alternates", v)}
                            />
                            <NumInput
                                label="alternates_sfs"
                                value={cfg().weights.alternates_sfs}
                                onChange={(v) => updateWeight("alternates_sfs", v)}
                            />
                            <NumInput
                                label="redirects"
                                value={cfg().weights.redirects}
                                onChange={(v) => updateWeight("redirects", v)}
                            />
                            <NumInput
                                label="redirects_sfs"
                                value={cfg().weights.redirects_sfs}
                                onChange={(v) => updateWeight("redirects_sfs", v)}
                            />
                            <NumInput
                                label="bad_redirects"
                                value={cfg().weights.bad_redirects}
                                onChange={(v) => updateWeight("bad_redirects", v)}
                            />
                            <NumInput
                                label="bad_redirects_sfs"
                                value={cfg().weights.bad_redirects_sfs}
                                onChange={(v) => updateWeight("bad_redirects_sfs", v)}
                            />
                        </section>

                        {/* Finger weights */}
                        <section class="border border-neutral-700 p-4 flex flex-col gap-2">
                            <div class="text-xs text-neutral-500 uppercase tracking-widest mb-1">
                                Finger Weights
                            </div>
                            <div class="grid grid-cols-2 gap-x-6 gap-y-1">
                                {(
                                    [
                                        ["lp", "Left pinky"],
                                        ["lr", "Left ring"],
                                        ["lm", "Left middle"],
                                        ["li", "Left index"],
                                        ["lt", "Left thumb"],
                                        ["rt", "Right thumb"],
                                        ["ri", "Right index"],
                                        ["rm", "Right middle"],
                                        ["rr", "Right ring"],
                                        ["rp", "Right pinky"],
                                    ] as [string, string][]
                                ).map(([key, label]) => (
                                    <NumInput
                                        label={label}
                                        value={(cfg().weights.finger_weights as any)[key]}
                                        step={0.1}
                                        onChange={(v) => updateFingerWeight(key, v)}
                                    />
                                ))}
                            </div>
                        </section>

                        {/* Max finger use */}
                        <section class="border border-neutral-700 p-4 flex flex-col gap-2">
                            <div class="text-xs text-neutral-500 uppercase tracking-widest mb-1">
                                Max Finger Use
                            </div>
                            <div class="grid grid-cols-2 gap-x-6 gap-y-1">
                                {(
                                    [
                                        ["penalty", "Penalty multiplier"],
                                        ["pinky", "Pinky max %"],
                                        ["ring", "Ring max %"],
                                        ["middle", "Middle max %"],
                                        ["index", "Index max %"],
                                        ["thumb", "Thumb max %"],
                                    ] as [string, string][]
                                ).map(([key, label]) => (
                                    <NumInput
                                        label={label}
                                        value={(cfg().weights.max_finger_use as any)[key]}
                                        step={0.5}
                                        onChange={(v) => updateMaxFingerUse(key, v)}
                                    />
                                ))}
                            </div>
                        </section>

                        {/* Presets */}
                        <section class="border border-neutral-700 p-4 flex flex-col gap-3">
                            <div class="text-xs text-neutral-500 uppercase tracking-widest">
                                Weight Presets
                            </div>
                            <Show when={presets().length > 0}>
                                <div class="flex gap-2 items-center">
                                    <label class="text-xs text-neutral-400 font-mono shrink-0">Load</label>
                                    <Dropdown class="flex-1" onChange={handleLoadPreset}>
                                        <option value="">— select preset —</option>
                                        <For each={presets()}>
                                            {(name) => <option value={name}>{name}</option>}
                                        </For>
                                    </Dropdown>
                                </div>
                            </Show>
                            <div class="flex gap-2">
                                <input
                                    class="bg-neutral-800 border border-neutral-600 text-neutral-100 font-mono text-sm px-2 py-1 w-44"
                                    placeholder="preset name…"
                                    value={presetName()}
                                    onInput={(e) => setPresetName(e.currentTarget.value)}
                                    onKeyDown={(e) => e.key === "Enter" && savePreset()}
                                />
                                <button
                                    class="border border-neutral-600 font-mono text-sm px-3 py-1 hover:bg-neutral-700"
                                    onClick={savePreset}
                                >
                                    Save preset
                                </button>
                            </div>
                        </section>

                        {/* Actions */}
                        <div class="flex gap-3 flex-wrap">
                            <button
                                class="border border-neutral-500 font-mono text-sm px-4 py-1.5 hover:bg-neutral-700 disabled:opacity-40"
                                disabled={saving()}
                                onClick={handleSave}
                            >
                                {saving() ? "Saving…" : "Save & Apply"}
                            </button>
                            <button
                                class="border border-neutral-600 font-mono text-sm px-4 py-1.5 hover:bg-neutral-700"
                                onClick={handleResetDefaults}
                            >
                                Reset to defaults
                            </button>
                            <button
                                class="border border-neutral-600 font-mono text-sm px-4 py-1.5 hover:bg-neutral-700"
                                onClick={exportWeights}
                            >
                                Copy weights to clipboard
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
                )}
            </Show>
        </div>
    );
}
