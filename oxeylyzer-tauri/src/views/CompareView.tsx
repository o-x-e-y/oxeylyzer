import { createSignal, Show } from "solid-js";
import KeyboardDisplay from "../components/KeyboardDisplay";
import { MOCK_LAYOUTS } from "../mock";
import type { Layout } from "../mock";

// ── Sub-components ────────────────────────────────────────────────────────────

function CRow(props: { label: string; v1: string; v2: string; dim?: boolean }) {
    const lc = () => (props.dim ? "text-neutral-500" : "text-neutral-400");
    const vc = () => (props.dim ? "text-neutral-500" : "text-neutral-100");
    return (
        <div class="flex gap-2 font-mono text-sm py-px">
            <span class={`w-36 shrink-0 ${lc()}`}>{props.label}</span>
            <span class={`w-20 text-right shrink-0 ${vc()}`}>{props.v1}</span>
            <span class={`w-20 text-right shrink-0 ${vc()}`}>{props.v2}</span>
        </div>
    );
}

function CHeader(props: { name1: string; name2: string }) {
    return (
        <div class="flex gap-2 font-mono text-xs text-neutral-500 pb-1 mb-0.5 border-b border-neutral-600">
            <span class="w-36 shrink-0" />
            <span class="w-20 text-right shrink-0 truncate">{props.name1}</span>
            <span class="w-20 text-right shrink-0 truncate">{props.name2}</span>
        </div>
    );
}

function CSep() {
    return <div class="border-t border-neutral-700 my-1.5" />;
}

// ── Main component ────────────────────────────────────────────────────────────

export default function CompareView() {
    const layoutNames = () => MOCK_LAYOUTS.map((l) => l.name);

    const [name1, setName1] = createSignal(MOCK_LAYOUTS[0].name);
    const [name2, setName2] = createSignal(MOCK_LAYOUTS[1].name);
    const [compared, setCompared] = createSignal<[Layout, Layout] | null>([
        MOCK_LAYOUTS[0],
        MOCK_LAYOUTS[1],
    ]);

    const getLayout = (name: string) =>
        MOCK_LAYOUTS.find((l) => l.name === name) ?? MOCK_LAYOUTS[0];

    const handleCompare = () => setCompared([getLayout(name1()), getLayout(name2())]);

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
                    <select
                        class="bg-neutral-800 border border-neutral-600 text-neutral-100 font-mono text-sm px-2 py-1"
                        value={name1()}
                        onChange={(e) => setName1(e.currentTarget.value)}
                    >
                        {layoutNames().map((n) => (
                            <option value={n}>{n}</option>
                        ))}
                    </select>
                </div>

                <div class="flex flex-col gap-1">
                    <label class="text-xs text-neutral-500 font-mono uppercase tracking-widest">
                        Layout 2
                    </label>
                    <select
                        class="bg-neutral-800 border border-neutral-600 text-neutral-100 font-mono text-sm px-2 py-1"
                        value={name2()}
                        onChange={(e) => setName2(e.currentTarget.value)}
                    >
                        {layoutNames().map((n) => (
                            <option value={n}>{n}</option>
                        ))}
                    </select>
                </div>

                <button
                    class="border border-neutral-500 text-neutral-100 font-mono text-sm px-4 py-1 hover:bg-neutral-700"
                    onClick={handleCompare}
                >
                    Compare
                </button>
            </div>

            {/* ── Compared content ─────────────────────────────────── */}
            <Show when={compared()} keyed>
                {([l1, l2]) => {
                    const s1 = l1.stats;
                    const s2 = l2.stats;
                    return (
                        <div class="flex flex-col gap-6">
                            {/* Keyboards */}
                            <div class="flex gap-16 items-start">
                                <div class="flex flex-col gap-2">
                                    <div class="font-mono text-neutral-200">{l1.name}</div>
                                    <KeyboardDisplay keys={l1.keys} />
                                </div>
                                <div class="flex flex-col gap-2">
                                    <div class="font-mono text-neutral-200">{l2.name}</div>
                                    <KeyboardDisplay keys={l2.keys} />
                                </div>
                            </div>

                            {/* Stats — two independent comparison columns */}
                            <div class="grid grid-cols-2 gap-x-16 items-start">
                                {/* ── Left col: Basic + Position Bigrams ── */}
                                <div>
                                    <CHeader name1={l1.name} name2={l2.name} />

                                    <CRow label="Sfb" v1={pct(s1.sfb)} v2={pct(s2.sfb)} />
                                    <CRow label="Dsfb" v1={pct(s1.dsfb)} v2={pct(s2.dsfb)} />
                                    <CRow label="Fspeed" v1={num(s1.fspeed)} v2={num(s2.fspeed)} />
                                    <CRow label="Score" v1={num(s1.score)} v2={num(s2.score)} />

                                    <CSep />

                                    <CRow
                                        label="Stretches"
                                        v1={pct(s1.stretches)}
                                        v2={pct(s2.stretches)}
                                    />
                                    <CRow
                                        label="Scissors"
                                        v1={pct(s1.scissors)}
                                        v2={pct(s2.scissors)}
                                    />
                                    <CRow label="LSBs" v1={pct(s1.lsbs)} v2={pct(s2.lsbs)} />
                                    <CRow
                                        label="Pinky-Ring"
                                        v1={pct(s1.pinky_ring)}
                                        v2={pct(s2.pinky_ring)}
                                    />
                                </div>

                                {/* ── Right col: Rolls + Alternation + Redirects + Misc ── */}
                                <div>
                                    <CHeader name1={l1.name} name2={l2.name} />

                                    <CRow
                                        label="Inrolls"
                                        v1={pct(s1.inrolls)}
                                        v2={pct(s2.inrolls)}
                                    />
                                    <CRow
                                        label="Outrolls"
                                        v1={pct(s1.outrolls)}
                                        v2={pct(s2.outrolls)}
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
                                    />

                                    <CSep />

                                    <CRow
                                        label="Alternates"
                                        v1={pct(s1.alternates)}
                                        v2={pct(s2.alternates)}
                                    />
                                    <CRow
                                        label="Alt. (sfs)"
                                        v1={pct(s1.alternates_sfs)}
                                        v2={pct(s2.alternates_sfs)}
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
                                    />
                                    <CRow
                                        label="Redir. Sfs"
                                        v1={pct(s1.redirects_sfs)}
                                        v2={pct(s2.redirects_sfs)}
                                    />
                                    <CRow
                                        label="Bad Redir."
                                        v1={pct(s1.bad_redirects)}
                                        v2={pct(s2.bad_redirects)}
                                    />
                                    <CRow
                                        label="Bad Redir. Sfs"
                                        v1={pct(s1.bad_redirects_sfs)}
                                        v2={pct(s2.bad_redirects_sfs)}
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
                                    />
                                    <CRow label="Sft" v1={pct(s1.sfts)} v2={pct(s2.sfts)} />
                                </div>
                            </div>
                        </div>
                    );
                }}
            </Show>
        </div>
    );
}
