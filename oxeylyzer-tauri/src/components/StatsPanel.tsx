import { For } from "solid-js";
import type { LayoutStats } from "../mock";

type Props = {
  stats: LayoutStats;
};

function StatRow(props: { label: string; value: string; dim?: boolean }) {
  return (
    <div class="flex justify-between gap-8 py-0.5">
      <span class={props.dim ? "text-neutral-500" : "text-neutral-400"}>
        {props.label}
      </span>
      <span class={props.dim ? "text-neutral-500" : "text-neutral-100"}>
        {props.value}
      </span>
    </div>
  );
}

function Section(props: { title: string; children: any }) {
  return (
    <div class="border border-neutral-700 p-3">
      <div class="text-xs text-neutral-500 uppercase tracking-widest mb-2">
        {props.title}
      </div>
      {props.children}
    </div>
  );
}

const FINGER_LABELS = ["LP", "LR", "LM", "LI", "LT", "RT", "RI", "RM", "RR", "RP"] as const;

export default function StatsPanel(props: Props) {
  const s = () => props.stats;
  const pct = (v: number) => `${v.toFixed(3)}%`;
  const num = (v: number) => v.toFixed(3);

  return (
    <div class="flex flex-col gap-2 text-sm font-mono">
      <Section title="Basic">
        <StatRow label="Sfb"          value={pct(s().sfb)} />
        <StatRow label="Dsfb"         value={pct(s().dsfb)} />
        <StatRow label="Finger Speed" value={num(s().fspeed)} />
        <StatRow label="Score"        value={num(s().score)} />
      </Section>

      <Section title="Finger Speed">
        <div class="flex gap-2 mb-1">
          <For each={FINGER_LABELS}>
            {(label) => (
              <div class="text-center" style={{ "min-width": "2.5rem" }}>
                <div class="text-neutral-500 text-xs">{label}</div>
              </div>
            )}
          </For>
        </div>
        <div class="flex gap-2">
          <For each={s().finger_speed}>
            {(v, i) => (
              <div
                class="text-center text-xs"
                style={{ "min-width": "2.5rem" }}
              >
                <span
                  class={
                    i() === 4 || i() === 5
                      ? "text-neutral-500"
                      : "text-neutral-200"
                  }
                >
                  {v.toFixed(2)}
                </span>
              </div>
            )}
          </For>
        </div>
      </Section>

      <Section title="Position Bigrams">
        <StatRow label="Stretches"         value={pct(s().stretches)} />
        <StatRow label="Scissors"          value={pct(s().scissors)} />
        <StatRow label="LSBs"              value={pct(s().lsbs)} />
        <StatRow label="Pinky-Ring"        value={pct(s().pinky_ring)} />
      </Section>

      <Section title="Rolls">
        <StatRow label="Inrolls"           value={pct(s().inrolls)} />
        <StatRow label="Outrolls"          value={pct(s().outrolls)} />
        <StatRow
          label="Total Rolls"
          value={pct(s().inrolls + s().outrolls)}
          dim
        />
        <StatRow label="Onehands"          value={pct(s().onehands)} />
      </Section>

      <Section title="Alternation">
        <StatRow label="Alternates"        value={pct(s().alternates)} />
        <StatRow label="Alternates (sfs)"  value={pct(s().alternates_sfs)} />
        <StatRow
          label="Total Alternates"
          value={pct(s().alternates + s().alternates_sfs)}
          dim
        />
      </Section>

      <Section title="Redirects">
        <StatRow label="Redirects"         value={pct(s().redirects)} />
        <StatRow label="Redirects Sfs"     value={pct(s().redirects_sfs)} />
        <StatRow label="Bad Redirects"     value={pct(s().bad_redirects)} />
        <StatRow label="Bad Redirects Sfs" value={pct(s().bad_redirects_sfs)} />
        <StatRow
          label="Total Redirects"
          value={pct(
            s().redirects +
              s().redirects_sfs +
              s().bad_redirects +
              s().bad_redirects_sfs,
          )}
          dim
        />
      </Section>

      <Section title="Misc">
        <StatRow label="Bad Sfbs" value={pct(s().bad_sfbs)} />
        <StatRow label="Sft"      value={pct(s().sfts)} />
      </Section>
    </div>
  );
}
