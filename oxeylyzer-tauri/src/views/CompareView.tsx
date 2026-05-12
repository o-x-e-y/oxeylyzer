import { createEffect, createSignal, Show } from "solid-js";
import LayoutSearch from "../components/LayoutSearch";
import KeyboardDisplay from "../components/KeyboardDisplay";
import { appStore } from "../store";
import { analyzeLayout } from "../api";
import type { Layout, LayoutStats } from "../mock";

// ── Sub-components ────────────────────────────────────────────────────────────

function delta(v1: number, v2: number, higherIsBetter = false): { text: string; color: string } {
    const d = v2 - v1;
    if (Math.abs(d) < 0.0005) return { text: "=", color: "text-neutral-500" };
    if (higherIsBetter) {
        return d > 0
            ? { text: `▲ +${d.toFixed(3)}`, color: "text-green-400" }
            : { text: `▼ ${d.toFixed(3)}`, color: "text-red-400" };
    }
    return d < 0
        ? { text: `▲ ${d.toFixed(3)}`, color: "text-green-400" }
        : { text: `▼ +${d.toFixed(3)}`, color: "text-red-400" };
}

function CRow(props: {
    label: string;
    v1: string;
    v2: string;
    d?: { text: string; color: string };
    dim?: boolean;
}) {
    const lc = () => (props.dim ? "text-neutral-500" : "text-neutral-400");
    const vc = () => (props.dim ? "text-neutral-500" : "text-neutral-100");
    return (
        <div class="flex gap-2 font-mono text-sm py-px">
            <span class={`w-36 shrink-0 ${lc()}`}>{props.label}</span>
            <span class={`w-20 text-right shrink-0 ${vc()}`}>{props.v1}</span>
            <span class={`w-20 text-right shrink-0 ${vc()}`}>{props.v2}</span>
            {props.d && (
                <span class={`w-24 text-right shrink-0 text-xs ${props.d.color}`}>
                    {props.d.text}
                </span>
            )}
        </div>
    );
}

function CHeader(props: { name1: string; name2: string }) {
    return (
        <div class="flex gap-2 font-mono text-xs text-neutral-500 pb-1 mb-0.5 border-b border-neutral-600">
            <span class="w-36 shrink-0" />
            <span class="w-20 text-right shrink-0 truncate">{props.name1}</span>
            <span class="w-20 text-right shrink-0 truncate">{props.name2}</span>
            <span class="w-24 text-right shrink-0">Δ (2-1)</span>
        </div>
    );
}

function CSep() {
    return <div class="border-t border-neutral-700 my-1.5" />;
}

// ── Main component ────────────────────────────────────────────────────────────

export default function CompareView() {
    const [name1, setName1] = createSignal(appStore.layouts[0]?.name ?? "");
    const [name2, setName2] = createSignal(appStore.layouts[1]?.name ?? "");
    const [compared, setCompared] = createSignal<[Layout, Layout] | null>(null);
    const [loading, setLoading] = createSignal(false);
    const [error, setError] = createSignal("");

    createEffect(() => {
        const n1 = name1(), n2 = name2();
        if (!n1 || !n2) return;
        setLoading(true);
        setError("");
        Promise.all([analyzeLayout(n1), analyzeLayout(n2)])
            .then(([l1, l2]) => setCompared([l1, l2]))
            .catch((e) => setError(String(e)))
            .finally(() => setLoading(false));
    });

    const pct = (v: number) => `${v.toFixed(3)}%`;
    const num = (v: number) => v.toFixed(3);
    const sum = (...vs: number[]) => `${vs.reduce((a, b) => a + b, 0).toFixed(3)}%`;

    return (
        <div class="flex-1 min-h-0 overflow-y-auto flex flex-col gap-4">
            <h1 class="text-lg font-mono text-neutral-300 shrink-0">Compare</h1>

            {/* ── Pickers ──────────────────────────────────────────── */}
            <div class="flex gap-3 items-end shrink-0">
                <div class="flex flex-col gap-1">
                    <label class="text-xs text-neutral-500 font-mono uppercase tracking-widest">
                        Layout 1
                    </label>
                    <LayoutSearch value={name1()} onSelect={setName1} />
                </div>
                <div class="flex flex-col gap-1">
                    <label class="text-xs text-neutral-500 font-mono uppercase tracking-widest">
                        Layout 2
                    </label>
                    <LayoutSearch value={name2()} onSelect={setName2} />
                </div>
                <Show when={loading()}>
                    <span class="text-neutral-500 text-sm font-mono pb-1">…</span>
                </Show>
            </div>

            <Show when={error()}>
                <div class="text-red-400 text-sm font-mono">{error()}</div>
            </Show>

            {/* ── Compared content ─────────────────────────────────── */}
            <Show when={compared()} keyed>
                {([l1, l2]) => {
                    const s1 = l1.stats as LayoutStats & Record<string, number>;
                    const s2 = l2.stats as LayoutStats & Record<string, number>;
                    return (
                        <div class="flex flex-col gap-6">
                            {/* Keyboards */}
                            <div class="flex gap-16 items-start">
                                <div class="flex flex-col gap-2">
                                    <div class="font-mono text-neutral-200">{l1.name}</div>
                                    <KeyboardDisplay
                                        keys={l1.keys}
                                        keyboard={l1.keyboard}
                                        shape={l1.shape}
                                        heatmap={appStore.charFrequencies}
                                    />
                                </div>
                                <div class="flex flex-col gap-2">
                                    <div class="font-mono text-neutral-200">{l2.name}</div>
                                    <KeyboardDisplay
                                        keys={l2.keys}
                                        keyboard={l2.keyboard}
                                        shape={l2.shape}
                                        heatmap={appStore.charFrequencies}
                                    />
                                </div>
                            </div>

                            {/* Stats — two comparison columns */}
                            <div class="grid grid-cols-2 gap-x-16 items-start">
                                {/* ── Left col ── */}
                                <div>
                                    <CHeader name1={l1.name} name2={l2.name} />
                                    <CRow
                                        label="Sfb"
                                        v1={pct(s1.sfb)}
                                        v2={pct(s2.sfb)}
                                        d={delta(s1.sfb, s2.sfb)}
                                    />
                                    <CRow
                                        label="Dsfb"
                                        v1={pct(s1.dsfb)}
                                        v2={pct(s2.dsfb)}
                                        d={delta(s1.dsfb, s2.dsfb)}
                                    />
                                    <CRow
                                        label="Fspeed"
                                        v1={num(s1.fspeed)}
                                        v2={num(s2.fspeed)}
                                        d={delta(s1.fspeed, s2.fspeed)}
                                    />
                                    <CRow
                                        label="Score"
                                        v1={num(s1.score)}
                                        v2={num(s2.score)}
                                        d={delta(s1.score, s2.score, true)}
                                    />
                                    <CSep />
                                    <CRow
                                        label="Stretches"
                                        v1={num(s1.stretches)}
                                        v2={num(s2.stretches)}
                                        d={delta(s1.stretches, s2.stretches)}
                                    />
                                    <CRow
                                        label="Scissors"
                                        v1={pct(s1.scissors)}
                                        v2={pct(s2.scissors)}
                                        d={delta(s1.scissors, s2.scissors)}
                                    />
                                    <CRow
                                        label="LSBs"
                                        v1={pct(s1.lsbs)}
                                        v2={pct(s2.lsbs)}
                                        d={delta(s1.lsbs, s2.lsbs)}
                                    />
                                    <CRow
                                        label="Pinky-Ring"
                                        v1={pct(s1.pinky_ring)}
                                        v2={pct(s2.pinky_ring)}
                                        d={delta(s1.pinky_ring, s2.pinky_ring)}
                                    />
                                </div>

                                {/* ── Right col ── */}
                                <div>
                                    <CHeader name1={l1.name} name2={l2.name} />
                                    <CRow
                                        label="Inrolls"
                                        v1={pct(s1.inrolls)}
                                        v2={pct(s2.inrolls)}
                                        d={delta(s1.inrolls, s2.inrolls, true)}
                                    />
                                    <CRow
                                        label="Outrolls"
                                        v1={pct(s1.outrolls)}
                                        v2={pct(s2.outrolls)}
                                        d={delta(s1.outrolls, s2.outrolls, true)}
                                    />
                                    <CRow
                                        label="Total Rolls"
                                        v1={sum(s1.inrolls, s1.outrolls)}
                                        v2={sum(s2.inrolls, s2.outrolls)}
                                        dim
                                    />
                                    <CRow
                                        label="Onehands"
                                        v1={pct(s1.onehands)}
                                        v2={pct(s2.onehands)}
                                        d={delta(s1.onehands, s2.onehands, true)}
                                    />
                                    <CSep />
                                    <CRow
                                        label="Alternates"
                                        v1={pct(s1.alternates)}
                                        v2={pct(s2.alternates)}
                                        d={delta(s1.alternates, s2.alternates, true)}
                                    />
                                    <CRow
                                        label="Alt. (sfs)"
                                        v1={pct(s1.alternates_sfs)}
                                        v2={pct(s2.alternates_sfs)}
                                        d={delta(s1.alternates_sfs, s2.alternates_sfs, true)}
                                    />
                                    <CRow
                                        label="Total Alt."
                                        v1={sum(s1.alternates, s1.alternates_sfs)}
                                        v2={sum(s2.alternates, s2.alternates_sfs)}
                                        dim
                                    />
                                    <CSep />
                                    <CRow
                                        label="Redirects"
                                        v1={pct(s1.redirects)}
                                        v2={pct(s2.redirects)}
                                        d={delta(s1.redirects, s2.redirects)}
                                    />
                                    <CRow
                                        label="Redir. Sfs"
                                        v1={pct(s1.redirects_sfs)}
                                        v2={pct(s2.redirects_sfs)}
                                        d={delta(s1.redirects_sfs, s2.redirects_sfs)}
                                    />
                                    <CRow
                                        label="Bad Redir."
                                        v1={pct(s1.bad_redirects)}
                                        v2={pct(s2.bad_redirects)}
                                        d={delta(s1.bad_redirects, s2.bad_redirects)}
                                    />
                                    <CRow
                                        label="Bad Redir. Sfs"
                                        v1={pct(s1.bad_redirects_sfs)}
                                        v2={pct(s2.bad_redirects_sfs)}
                                        d={delta(s1.bad_redirects_sfs, s2.bad_redirects_sfs)}
                                    />
                                    <CRow
                                        label="Total Redir."
                                        v1={sum(
                                            s1.redirects,
                                            s1.redirects_sfs,
                                            s1.bad_redirects,
                                            s1.bad_redirects_sfs,
                                        )}
                                        v2={sum(
                                            s2.redirects,
                                            s2.redirects_sfs,
                                            s2.bad_redirects,
                                            s2.bad_redirects_sfs,
                                        )}
                                        dim
                                    />
                                    <CSep />
                                    <CRow
                                        label="Bad Sfbs"
                                        v1={pct(s1.bad_sfbs)}
                                        v2={pct(s2.bad_sfbs)}
                                        d={delta(s1.bad_sfbs, s2.bad_sfbs)}
                                    />
                                    <CRow
                                        label="Sft"
                                        v1={pct(s1.sfts)}
                                        v2={pct(s2.sfts)}
                                        d={delta(s1.sfts, s2.sfts)}
                                    />
                                </div>
                            </div>
                        </div>
                    );
                }}
            </Show>
        </div>
    );
}
