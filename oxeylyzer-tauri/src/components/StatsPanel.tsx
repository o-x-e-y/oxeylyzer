import { For } from "solid-js";
import type { LayoutStats } from "../mock";

type Props = { stats: LayoutStats };

function Stat(props: { label: string; value: string; dim?: boolean }) {
    return (
        <div class="flex justify-between gap-2 py-px">
            <span class={props.dim ? "text-neutral-500" : "text-neutral-400"}>{props.label}</span>
            <span class={props.dim ? "text-neutral-500" : "text-neutral-100"}>{props.value}</span>
        </div>
    );
}

function Sep() {
    return <div class="col-span-2 border-t border-neutral-700 my-1.5" />;
}

export default function StatsPanel(props: Props) {
    const s = () => props.stats;
    const pct = (v: number) => `${v.toFixed(3)}%`;
    const num = (v: number) => v.toFixed(3);
    const sum = (...vs: number[]) => `${vs.reduce((a, b) => a + b, 0).toFixed(3)}%`;

    return (
        <div class="grid grid-cols-2 gap-x-6 font-mono text-sm">
            {/* ── Basic ─────────────────────────────────── */}
            <Stat label="Sfb" value={pct(s().sfb)} />
            <Stat label="Dsfb" value={pct(s().dsfb)} />
            <Stat label="Fspeed" value={num(s().fspeed)} />
            <Stat label="Score" value={num(s().score)} />

            <Sep />

            {/* ── Finger Speed ──────────────────────────── */}
            <div class="col-span-2 flex flex-col gap-1 pb-0.5">
                <div class="text-xs text-neutral-500 uppercase tracking-widest">Finger Speed</div>
                <div class="flex gap-1">
                    <For each={["LP", "LR", "LM", "LI", "LT"]}>
                        {(label, i) => (
                            <div class="text-center w-10 text-xs">
                                <div class="text-neutral-500">{label}</div>
                                <div>{s().finger_speed[i()].toFixed(2)}</div>
                            </div>
                        )}
                    </For>
                </div>
                <div class="flex gap-1">
                    <For each={["RT", "RI", "RM", "RR", "RP"]}>
                        {(label, i) => (
                            <div class="text-center w-10 text-xs">
                                <div class="text-neutral-500">{label}</div>
                                <div>{s().finger_speed[5 + i()].toFixed(2)}</div>
                            </div>
                        )}
                    </For>
                </div>
            </div>

            <Sep />

            {/* ── Position Bigrams ──────────────────────── */}
            <Stat label="Stretches" value={pct(s().stretches)} />
            <Stat label="Scissors" value={pct(s().scissors)} />
            <Stat label="LSBs" value={pct(s().lsbs)} />
            <Stat label="Pinky-Ring" value={pct(s().pinky_ring)} />

            <Sep />

            {/* ── Rolls ─────────────────────────────────── */}
            <Stat label="Inrolls" value={pct(s().inrolls)} />
            <Stat label="Outrolls" value={pct(s().outrolls)} />
            <Stat label="Total Rolls" value={sum(s().inrolls, s().outrolls)} dim />
            <Stat label="Onehands" value={pct(s().onehands)} />

            <Sep />

            {/* ── Alternation ───────────────────────────── */}
            <Stat label="Alternates" value={pct(s().alternates)} />
            <Stat label="Alt. (sfs)" value={pct(s().alternates_sfs)} />
            <Stat label="Total Alt." value={sum(s().alternates, s().alternates_sfs)} dim />
            <div />

            <Sep />

            {/* ── Redirects ─────────────────────────────── */}
            <Stat label="Redirects" value={pct(s().redirects)} />
            <Stat label="Redir. Sfs" value={pct(s().redirects_sfs)} />
            <Stat label="Bad Redir." value={pct(s().bad_redirects)} />
            <Stat label="Bad Redir. Sfs" value={pct(s().bad_redirects_sfs)} />
            <Stat
                label="Total Redir."
                value={sum(
                    s().redirects,
                    s().redirects_sfs,
                    s().bad_redirects,
                    s().bad_redirects_sfs,
                )}
                dim
            />
            <div />

            <Sep />

            {/* ── Misc ──────────────────────────────────── */}
            <Stat label="Bad Sfbs" value={pct(s().bad_sfbs)} />
            <Stat label="Sft" value={pct(s().sfts)} />
        </div>
    );
}
