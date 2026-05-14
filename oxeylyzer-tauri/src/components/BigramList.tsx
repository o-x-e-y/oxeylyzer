import { For, Show } from "solid-js";
import type { BigramEntry } from "../types";

type Props = {
  entries: BigramEntry[];
  count?: number;
  columns?: 1 | 2;
  onHoverBigram?: (_chars: string[]) => void;
  onLeave?: () => void;
};

function Entry(props: {
  n: number;
  bigram: string;
  percent: number;
  onMouseEnter?: () => void;
  onMouseLeave?: () => void;
}) {
  return (
    <div
      class="flex gap-3 py-[1px] font-mono text-sm hover:bg-neutral-800 rounded-sm cursor-default"
      onMouseEnter={props.onMouseEnter}
      onMouseLeave={props.onMouseLeave}
    >
      <span class="text-neutral-500 w-5 text-right shrink-0">{props.n}.</span>
      <span class="w-8 shrink-0">{props.bigram}</span>
      <span>{props.percent.toFixed(3)}%</span>
    </div>
  );
}

export default function BigramList(props: Props) {
  const visible = () => props.entries.slice(0, props.count ?? props.entries.length);
  const half = () => Math.ceil(visible().length / 2);
  const twoCol = () => (props.columns ?? 1) === 2;

  const handleEnter = (bigram: string) => {
    props.onHoverBigram?.(bigram.split(""));
  };

  const renderEntry = (entry: BigramEntry, n: number) => (
    <Entry
      n={n}
      bigram={entry.bigram}
      percent={entry.percent}
      onMouseEnter={() => handleEnter(entry.bigram)}
      onMouseLeave={() => props.onLeave?.()}
    />
  );

  return (
    <Show
      when={twoCol()}
      fallback={
        <div>
          <For each={visible()}>{(entry, i) => renderEntry(entry, i() + 1)}</For>
        </div>
      }
    >
      <div class="grid grid-cols-2 gap-x-6">
        <div>
          <For each={visible().slice(0, half())}>{(entry, i) => renderEntry(entry, i() + 1)}</For>
        </div>
        <div>
          <For each={visible().slice(half())}>
            {(entry, i) => renderEntry(entry, half() + i() + 1)}
          </For>
        </div>
      </div>
    </Show>
  );
}
