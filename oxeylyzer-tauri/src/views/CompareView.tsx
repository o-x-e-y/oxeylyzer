import { createSignal, Show } from "solid-js";
import KeyboardDisplay from "../components/KeyboardDisplay";
import { MOCK_LAYOUTS } from "../mock";
import type { Layout } from "../mock";

function CompareStatRow(props: { label: string; v1: string; v2: string; dim?: boolean }) {
    return (
        <div class="flex gap-2 py-0.5 text-sm font-mono">
            <span class={`w-44 text-right ${props.dim ? "text-neutral-500" : "text-neutral-400"}`}>
                {props.label}
            </span>
            <span class={`w-20 text-right ${props.dim ? "text-neutral-500" : "text-neutral-100"}`}>
                {props.v1}
            </span>
            <span class={`w-20 text-right ${props.dim ? "text-neutral-500" : "text-neutral-100"}`}>
                {props.v2}
            </span>
        </div>
    );
}

function StatSection(props: { title: string; children: any }) {
    return (
        <div class="mb-3">
            <div class="text-xs text-neutral-500 uppercase tracking-widest mb-1 pl-46">
                {props.title}
            </div>
            {props.children}
        </div>
    );
}

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

    const handleCompare = () => {
        setCompared([getLayout(name1()), getLayout(name2())]);
    };

    const pct = (v: number) => `${v.toFixed(3)}%`;
    const num = (v: number) => v.toFixed(3);

    return (
        <div class="flex flex-col gap-4 max-w-4xl">
            <h1 class="text-lg font-mono text-neutral-300">Compare</h1>

            {/* Pickers */}
            <div class="flex gap-3 items-end">
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

            <Show when={compared()} keyed>
                {([l1, l2]) => (
                    <div class="flex flex-col gap-6">
                        {/* Keyboards side by side */}
                        <div class="flex gap-16">
                            <div class="flex flex-col gap-2">
                                <div class="font-mono text-neutral-200">{l1.name}</div>
                                <KeyboardDisplay keys={l1.keys} />
                            </div>
                            <div class="flex flex-col gap-2">
                                <div class="font-mono text-neutral-200">{l2.name}</div>
                                <KeyboardDisplay keys={l2.keys} />
                            </div>
                        </div>

                        {/* Stats comparison */}
                        <div class="border border-neutral-700 p-4">
                            {/* Header row */}
                            <div class="flex gap-2 mb-3 text-xs text-neutral-500 font-mono uppercase tracking-widest">
                                <span class="w-44 text-right">Metric</span>
                                <span class="w-20 text-right">{l1.name}</span>
                                <span class="w-20 text-right">{l2.name}</span>
                            </div>

                            <StatSection title="Basic">
                                <CompareStatRow
                                    label="Sfb"
                                    v1={pct(l1.stats.sfb)}
                                    v2={pct(l2.stats.sfb)}
                                />
                                <CompareStatRow
                                    label="Dsfb"
                                    v1={pct(l1.stats.dsfb)}
                                    v2={pct(l2.stats.dsfb)}
                                />
                                <CompareStatRow
                                    label="Finger Speed"
                                    v1={num(l1.stats.fspeed)}
                                    v2={num(l2.stats.fspeed)}
                                />
                                <CompareStatRow
                                    label="Score"
                                    v1={num(l1.stats.score)}
                                    v2={num(l2.stats.score)}
                                />
                            </StatSection>

                            <StatSection title="Position Bigrams">
                                <CompareStatRow
                                    label="Stretches"
                                    v1={pct(l1.stats.stretches)}
                                    v2={pct(l2.stats.stretches)}
                                />
                                <CompareStatRow
                                    label="Scissors"
                                    v1={pct(l1.stats.scissors)}
                                    v2={pct(l2.stats.scissors)}
                                />
                                <CompareStatRow
                                    label="LSBs"
                                    v1={pct(l1.stats.lsbs)}
                                    v2={pct(l2.stats.lsbs)}
                                />
                                <CompareStatRow
                                    label="Pinky-Ring"
                                    v1={pct(l1.stats.pinky_ring)}
                                    v2={pct(l2.stats.pinky_ring)}
                                />
                            </StatSection>

                            <StatSection title="Rolls">
                                <CompareStatRow
                                    label="Inrolls"
                                    v1={pct(l1.stats.inrolls)}
                                    v2={pct(l2.stats.inrolls)}
                                />
                                <CompareStatRow
                                    label="Outrolls"
                                    v1={pct(l1.stats.outrolls)}
                                    v2={pct(l2.stats.outrolls)}
                                />
                                <CompareStatRow
                                    label="Total Rolls"
                                    v1={pct(l1.stats.inrolls + l1.stats.outrolls)}
                                    v2={pct(l2.stats.inrolls + l2.stats.outrolls)}
                                    dim
                                />
                                <CompareStatRow
                                    label="Onehands"
                                    v1={pct(l1.stats.onehands)}
                                    v2={pct(l2.stats.onehands)}
                                />
                            </StatSection>

                            <StatSection title="Alternation">
                                <CompareStatRow
                                    label="Alternates"
                                    v1={pct(l1.stats.alternates)}
                                    v2={pct(l2.stats.alternates)}
                                />
                                <CompareStatRow
                                    label="Alternates (sfs)"
                                    v1={pct(l1.stats.alternates_sfs)}
                                    v2={pct(l2.stats.alternates_sfs)}
                                />
                                <CompareStatRow
                                    label="Total Alternates"
                                    v1={pct(l1.stats.alternates + l1.stats.alternates_sfs)}
                                    v2={pct(l2.stats.alternates + l2.stats.alternates_sfs)}
                                    dim
                                />
                            </StatSection>

                            <StatSection title="Redirects">
                                <CompareStatRow
                                    label="Redirects"
                                    v1={pct(l1.stats.redirects)}
                                    v2={pct(l2.stats.redirects)}
                                />
                                <CompareStatRow
                                    label="Redirects Sfs"
                                    v1={pct(l1.stats.redirects_sfs)}
                                    v2={pct(l2.stats.redirects_sfs)}
                                />
                                <CompareStatRow
                                    label="Bad Redirects"
                                    v1={pct(l1.stats.bad_redirects)}
                                    v2={pct(l2.stats.bad_redirects)}
                                />
                                <CompareStatRow
                                    label="Bad Redirects Sfs"
                                    v1={pct(l1.stats.bad_redirects_sfs)}
                                    v2={pct(l2.stats.bad_redirects_sfs)}
                                />
                                <CompareStatRow
                                    label="Total Redirects"
                                    v1={pct(
                                        l1.stats.redirects +
                                            l1.stats.redirects_sfs +
                                            l1.stats.bad_redirects +
                                            l1.stats.bad_redirects_sfs,
                                    )}
                                    v2={pct(
                                        l2.stats.redirects +
                                            l2.stats.redirects_sfs +
                                            l2.stats.bad_redirects +
                                            l2.stats.bad_redirects_sfs,
                                    )}
                                    dim
                                />
                            </StatSection>

                            <StatSection title="Misc">
                                <CompareStatRow
                                    label="Bad Sfbs"
                                    v1={pct(l1.stats.bad_sfbs)}
                                    v2={pct(l2.stats.bad_sfbs)}
                                />
                                <CompareStatRow
                                    label="Sft"
                                    v1={pct(l1.stats.sfts)}
                                    v2={pct(l2.stats.sfts)}
                                />
                            </StatSection>
                        </div>
                    </div>
                )}
            </Show>
        </div>
    );
}
