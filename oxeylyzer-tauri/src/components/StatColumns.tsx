import { For, createMemo } from "solid-js";
import type { LayoutStats } from "../mock";

// ── Shared helpers ──────────────────────────────────────────────────────────

const pct = (v: number) => `${v.toFixed(3)}%`;
const num = (v: number) => v.toFixed(3);
const sum = (...vs: number[]) => `${vs.reduce((a: number, b: number) => a + b, 0).toFixed(3)}%`;

// showEqual: CompareView shows "=" for identical values; AnalyzeView omits it (space reserved).
export function calcDelta(
  v1: number,
  v2: number,
  higherIsBetter = false,
  showEqual = false,
): { text: string; color: string } | null {
  const d = v2 - v1;
  if (Math.abs(d) < 0.0005) {
    return showEqual ? { text: "=", color: "text-neutral-500" } : null;
  }
  const better = higherIsBetter ? d > 0 : d < 0;
  return {
    text: d > 0 ? `▲ +${d.toFixed(3)}` : `▼ ${d.toFixed(3)}`,
    color: better ? "text-green-400" : "text-red-400",
  };
}

// ── Shared separator ────────────────────────────────────────────────────────

export function StatSep() {
  return <div class="border-t border-neutral-700 my-1.5" />;
}

// ── Analyze row: label | value | delta (space always reserved) ──────────────

export function ARow(props: {
  label: string;
  value: string;
  delta?: { text: string; color: string } | null;
  dim?: boolean;
}) {
  const lc = () => (props.dim ? "text-neutral-500" : "text-neutral-400");
  const vc = () => (props.dim ? "text-neutral-500" : "text-neutral-100");
  return (
    <div class="flex gap-2 font-mono text-sm py-px">
      <span class={`w-36 shrink-0 ${lc()}`}>{props.label}</span>
      <span class={`w-20 text-right shrink-0 ${vc()}`}>{props.value}</span>
      <span
        class={`w-24 text-right shrink-0 text-xs ${props.delta ? props.delta.color : "invisible"}`}
      >
        {props.delta?.text ?? " "}
      </span>
    </div>
  );
}

// ── Compare row: label | v1 | v2 | delta ───────────────────────────────────

export function CmpRow(props: {
  label: string;
  v1: string;
  v2: string;
  delta?: { text: string; color: string } | null;
  dim?: boolean;
}) {
  const lc = () => (props.dim ? "text-neutral-500" : "text-neutral-400");
  const vc = () => (props.dim ? "text-neutral-500" : "text-neutral-100");
  return (
    <div class="flex gap-2 font-mono text-sm py-px">
      <span class={`w-36 shrink-0 ${lc()}`}>{props.label}</span>
      <span class={`w-20 text-right shrink-0 ${vc()}`}>{props.v1}</span>
      <span class={`w-20 text-right shrink-0 ${vc()}`}>{props.v2}</span>
      {props.delta && (
        <span class={`w-24 text-right shrink-0 text-xs ${props.delta.color}`}>
          {props.delta.text}
        </span>
      )}
    </div>
  );
}

// ── Compare header ──────────────────────────────────────────────────────────

export function CmpHeader(props: { name1: string; name2: string }) {
  return (
    <div class="flex gap-2 font-mono text-xs text-neutral-500 pb-1 mb-0.5 border-b border-neutral-600">
      <span class="w-36 shrink-0" />
      <span class="w-20 text-right shrink-0 truncate">{props.name1}</span>
      <span class="w-20 text-right shrink-0 truncate">{props.name2}</span>
      <span class="w-24 text-right shrink-0">Δ (2-1)</span>
    </div>
  );
}

// ── Stat column data (shared between both views) ────────────────────────────

type StatEntry =
  | { kind: "sep" }
  | {
      kind: "stat";
      label: string;
      getValue: (s: LayoutStats) => string;
      getNum?: (s: LayoutStats) => number;
      higherIsBetter?: boolean;
      dim?: boolean;
    };

const LEFT: StatEntry[] = [
  { kind: "stat", label: "Sfb", getValue: (s) => pct(s.sfb), getNum: (s) => s.sfb },
  { kind: "stat", label: "Dsfb", getValue: (s) => pct(s.dsfb), getNum: (s) => s.dsfb },
  { kind: "stat", label: "Fspeed", getValue: (s) => num(s.fspeed), getNum: (s) => s.fspeed },
  {
    kind: "stat",
    label: "Score",
    getValue: (s) => num(s.score),
    getNum: (s) => s.score,
    higherIsBetter: true,
  },
  { kind: "sep" },
  {
    kind: "stat",
    label: "Stretches",
    getValue: (s) => num(s.stretches),
    getNum: (s) => s.stretches,
  },
  { kind: "stat", label: "Scissors", getValue: (s) => pct(s.scissors), getNum: (s) => s.scissors },
  { kind: "stat", label: "LSBs", getValue: (s) => pct(s.lsbs), getNum: (s) => s.lsbs },
  {
    kind: "stat",
    label: "Pinky-Ring",
    getValue: (s) => pct(s.pinky_ring),
    getNum: (s) => s.pinky_ring,
  },
];

const RIGHT: StatEntry[] = [
  {
    kind: "stat",
    label: "Inrolls",
    getValue: (s) => pct(s.inrolls),
    getNum: (s) => s.inrolls,
    higherIsBetter: true,
  },
  {
    kind: "stat",
    label: "Outrolls",
    getValue: (s) => pct(s.outrolls),
    getNum: (s) => s.outrolls,
    higherIsBetter: true,
  },
  {
    kind: "stat",
    label: "Total Rolls",
    getValue: (s) => sum(s.inrolls, s.outrolls),
    dim: true,
  },
  {
    kind: "stat",
    label: "Onehands",
    getValue: (s) => pct(s.onehands),
    getNum: (s) => s.onehands,
    higherIsBetter: true,
  },
  { kind: "sep" },
  {
    kind: "stat",
    label: "Alternates",
    getValue: (s) => pct(s.alternates),
    getNum: (s) => s.alternates,
    higherIsBetter: true,
  },
  {
    kind: "stat",
    label: "Alt. (sfs)",
    getValue: (s) => pct(s.alternates_sfs),
    getNum: (s) => s.alternates_sfs,
    higherIsBetter: true,
  },
  {
    kind: "stat",
    label: "Total Alt.",
    getValue: (s) => sum(s.alternates, s.alternates_sfs),
    dim: true,
  },
  { kind: "sep" },
  {
    kind: "stat",
    label: "Redirects",
    getValue: (s) => pct(s.redirects),
    getNum: (s) => s.redirects,
  },
  {
    kind: "stat",
    label: "Redir. Sfs",
    getValue: (s) => pct(s.redirects_sfs),
    getNum: (s) => s.redirects_sfs,
  },
  {
    kind: "stat",
    label: "Bad Redir.",
    getValue: (s) => pct(s.bad_redirects),
    getNum: (s) => s.bad_redirects,
  },
  {
    kind: "stat",
    label: "Bad Redir. Sfs",
    getValue: (s) => pct(s.bad_redirects_sfs),
    getNum: (s) => s.bad_redirects_sfs,
  },
  {
    kind: "stat",
    label: "Total Redir.",
    getValue: (s) =>
      sum(s.redirects, s.redirects_sfs, s.bad_redirects, s.bad_redirects_sfs),
    dim: true,
  },
  { kind: "sep" },
  { kind: "stat", label: "Bad Sfbs", getValue: (s) => pct(s.bad_sfbs), getNum: (s) => s.bad_sfbs },
  { kind: "stat", label: "Sft", getValue: (s) => pct(s.sfts), getNum: (s) => s.sfts },
];

// ── Per-row components (proper SolidJS components so prop getters are reactive) ──

function AnalyzeRow(rowProps: {
  entry: Exclude<StatEntry, { kind: "sep" }>;
  stats: LayoutStats;
  baseline?: LayoutStats;
}) {
  const d = createMemo(() => {
    if (!rowProps.entry.getNum || !rowProps.baseline) return null;
    return calcDelta(
      rowProps.entry.getNum(rowProps.baseline),
      rowProps.entry.getNum(rowProps.stats),
      rowProps.entry.higherIsBetter,
    );
  });
  return (
    <ARow
      label={rowProps.entry.label}
      value={rowProps.entry.getValue(rowProps.stats)}
      delta={d()}
      dim={rowProps.entry.dim}
    />
  );
}

function CompareRow(rowProps: {
  entry: Exclude<StatEntry, { kind: "sep" }>;
  s1: LayoutStats;
  s2: LayoutStats;
}) {
  const d = createMemo(() => {
    if (!rowProps.entry.getNum) return undefined;
    return (
      calcDelta(
        rowProps.entry.getNum(rowProps.s1),
        rowProps.entry.getNum(rowProps.s2),
        rowProps.entry.higherIsBetter,
        true,
      ) ?? undefined
    );
  });
  return (
    <CmpRow
      label={rowProps.entry.label}
      v1={rowProps.entry.getValue(rowProps.s1)}
      v2={rowProps.entry.getValue(rowProps.s2)}
      delta={d()}
      dim={rowProps.entry.dim}
    />
  );
}

// ── AnalyzeStatColumns ──────────────────────────────────────────────────────

export function AnalyzeStatColumns(props: { stats: LayoutStats; baseline?: LayoutStats }) {
  const renderEntry = (entry: StatEntry) => {
    if (entry.kind === "sep") return <StatSep />;
    return <AnalyzeRow entry={entry} stats={props.stats} baseline={props.baseline} />;
  };

  return (
    <div class="grid grid-cols-2 gap-x-16 items-start">
      <div>
        <For each={LEFT}>{renderEntry}</For>
      </div>
      <div>
        <For each={RIGHT}>{renderEntry}</For>
      </div>
    </div>
  );
}

// ── CompareStatColumns ──────────────────────────────────────────────────────

export function CompareStatColumns(props: {
  s1: LayoutStats;
  s2: LayoutStats;
  name1: string;
  name2: string;
}) {
  const renderEntry = (entry: StatEntry) => {
    if (entry.kind === "sep") return <StatSep />;
    return <CompareRow entry={entry} s1={props.s1} s2={props.s2} />;
  };

  return (
    <div class="grid grid-cols-2 gap-x-16 items-start">
      <div>
        <CmpHeader name1={props.name1} name2={props.name2} />
        <For each={LEFT}>{renderEntry}</For>
      </div>
      <div>
        <CmpHeader name1={props.name1} name2={props.name2} />
        <For each={RIGHT}>{renderEntry}</For>
      </div>
    </div>
  );
}
