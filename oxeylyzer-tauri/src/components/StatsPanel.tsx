import { For, Show } from "solid-js";
import type { LayoutStats } from "../mock";

type Props = { stats: LayoutStats; baseline?: LayoutStats };

function diff(
  current: number,
  baseline: number,
  higherIsBetter = false,
): { text: string; color: string } | null {
  const d = current - baseline;
  if (Math.abs(d) < 0.00005) return null;
  const better = higherIsBetter ? d > 0 : d < 0;
  return {
    text: `${d > 0 ? "+" : ""}${d.toFixed(3)}`,
    color: better ? "text-green-400" : "text-red-400",
  };
}

function Stat(props: {
  label: string;
  value: string;
  delta?: { text: string; color: string } | null;
  dim?: boolean;
}) {
  return (
    <div class="flex justify-between gap-2 py-px">
      <span class={props.dim ? "text-neutral-500" : "text-neutral-400"}>{props.label}</span>
      <div class="flex items-center gap-1.5">
        <Show when={props.delta}>
          {(d) => <span class={`text-xs tabular-nums ${d().color}`}>{d().text}</span>}
        </Show>
        <span class={props.dim ? "text-neutral-500" : "text-neutral-100"}>{props.value}</span>
      </div>
    </div>
  );
}

function Sep() {
  return <div class="col-span-2 border-t border-neutral-700 my-1.5" />;
}

export default function StatsPanel(props: Props) {
  const s = () => props.stats;
  const b = () => props.baseline;
  const pct = (v: number) => `${v.toFixed(3)}%`;
  const num = (v: number) => v.toFixed(3);
  const sum = (...vs: number[]) => `${vs.reduce((a, c) => a + c, 0).toFixed(3)}%`;

  const d = (get: (_s: LayoutStats) => number, higher = false) => {
    const bl = b();
    return bl ? diff(get(s()), get(bl), higher) : null;
  };

  return (
    <div class="grid grid-cols-2 gap-x-6 font-mono text-sm">
      <Stat label="Sfb" value={pct(s().sfb)} delta={d((s) => s.sfb)} />
      <Stat label="Dsfb" value={pct(s().dsfb)} delta={d((s) => s.dsfb)} />
      <Stat label="Fspeed" value={num(s().fspeed)} delta={d((s) => s.fspeed)} />
      <Stat label="Score" value={num(s().score)} delta={d((s) => s.score, true)} />

      <Sep />

      <div class="col-span-2 flex flex-col gap-1 pb-0.5">
        <div class="text-xs text-neutral-500 uppercase tracking-widest">Finger Speed</div>
        <div class="flex gap-1">
          <For each={["LP", "LR", "LM", "LI", "LT"]}>
            {(label, i) => (
              <div class="text-center w-10 text-xs">
                <div class="text-neutral-500">{label}</div>
                <div>{(s().finger_speed[i()] * 10).toFixed(2)}</div>
              </div>
            )}
          </For>
        </div>
        <div class="flex gap-1">
          <For each={["RT", "RI", "RM", "RR", "RP"]}>
            {(label, i) => (
              <div class="text-center w-10 text-xs">
                <div class="text-neutral-500">{label}</div>
                <div>{(s().finger_speed[5 + i()] * 10).toFixed(2)}</div>
              </div>
            )}
          </For>
        </div>
      </div>

      <Sep />

      <Stat label="Stretches" value={num(s().stretches)} delta={d((s) => s.stretches)} />
      <Stat label="Scissors" value={pct(s().scissors)} delta={d((s) => s.scissors)} />
      <Stat label="LSBs" value={pct(s().lsbs)} delta={d((s) => s.lsbs)} />
      <Stat label="Pinky-Ring" value={pct(s().pinky_ring)} delta={d((s) => s.pinky_ring)} />

      <Sep />

      <Stat label="Inrolls" value={pct(s().inrolls)} delta={d((s) => s.inrolls, true)} />
      <Stat label="Outrolls" value={pct(s().outrolls)} delta={d((s) => s.outrolls, true)} />
      <Stat label="Total Rolls" value={sum(s().inrolls, s().outrolls)} dim />
      <Stat label="Onehands" value={pct(s().onehands)} delta={d((s) => s.onehands, true)} />

      <Sep />

      <Stat label="Alternates" value={pct(s().alternates)} delta={d((s) => s.alternates, true)} />
      <Stat
        label="Alt. (sfs)"
        value={pct(s().alternates_sfs)}
        delta={d((s) => s.alternates_sfs, true)}
      />
      <Stat label="Total Alt." value={sum(s().alternates, s().alternates_sfs)} dim />
      <div />

      <Sep />

      <Stat label="Redirects" value={pct(s().redirects)} delta={d((s) => s.redirects)} />
      <Stat label="Redir. Sfs" value={pct(s().redirects_sfs)} delta={d((s) => s.redirects_sfs)} />
      <Stat label="Bad Redir." value={pct(s().bad_redirects)} delta={d((s) => s.bad_redirects)} />
      <Stat
        label="Bad Redir. Sfs"
        value={pct(s().bad_redirects_sfs)}
        delta={d((s) => s.bad_redirects_sfs)}
      />
      <Stat
        label="Total Redir."
        value={sum(s().redirects, s().redirects_sfs, s().bad_redirects, s().bad_redirects_sfs)}
        dim
      />
      <div />

      <Sep />

      <Stat label="Bad Sfbs" value={pct(s().bad_sfbs)} delta={d((s) => s.bad_sfbs)} />
      <Stat label="Sft" value={pct(s().sfts)} delta={d((s) => s.sfts)} />
    </div>
  );
}
