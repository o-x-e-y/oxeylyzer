import { For } from "solid-js";
import type { BigramEntry } from "../mock";

type Props = {
  entries: BigramEntry[];
  count?: number;
};

export default function BigramList(props: Props) {
  const visible = () => props.entries.slice(0, props.count ?? props.entries.length);

  return (
    <div class="font-mono text-sm">
      <For each={visible()}>
        {(entry, i) => (
          <div class="flex gap-3 py-0.5">
            <span class="text-neutral-500 w-5 text-right">{i() + 1}.</span>
            <span class="w-8">{entry.bigram}</span>
            <span>{entry.percent.toFixed(3)}%</span>
          </div>
        )}
      </For>
    </div>
  );
}
