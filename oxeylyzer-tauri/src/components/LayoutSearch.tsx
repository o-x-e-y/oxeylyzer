import { createSignal, For, Show } from "solid-js";
import { appStore } from "../store";

type Props = {
  value: string;
  onSelect: (_name: string) => void;
  placeholder?: string;
  class?: string;
};

function* trigrams(str: string): Generator<string> {
  if (!str) return;
  const padded = "  " + str.toLowerCase() + " ";
  for (let i = 0; i < padded.length - 2; i++) yield padded[i] + padded[i + 1] + padded[i + 2];
}

function searchLayouts(query: string, names: string[], max = 7): string[] {
  const qt = [...trigrams(query)];
  if (!qt.length) return [];
  const scores: Record<string, number> = {};
  for (const name of names) {
    for (const nt of trigrams(name)) {
      for (const q of qt) {
        if (nt === q) scores[name] = (scores[name] ?? 0) + 1 / name.length;
      }
    }
  }
  return Object.entries(scores)
    .sort((a, b) => b[1] - a[1])
    .slice(0, max)
    .map(([n]) => n);
}

export default function LayoutSearch(props: Props) {
  const [query, setQuery] = createSignal("");
  const [open, setOpen] = createSignal(false);
  const [results, setResults] = createSignal<string[]>([]);
  const [selectedIdx, setSelectedIdx] = createSignal(0);
  const [hovering, setHovering] = createSignal(false);
  let input!: HTMLInputElement;

  const names = () => appStore.layouts.map((l) => l.name);
  const allSorted = () => [...names()].sort((a, b) => a.localeCompare(b));

  const close = () => {
    setOpen(false);
    setResults([]);
    setQuery("");
  };

  const doSearch = (q: string) => {
    // Empty query shows the full (alphabetical) list so the user can browse
    // before typing; queries narrow it via trigram matching.
    const found = q ? searchLayouts(q, names()) : allSorted();
    setResults(found);
    setSelectedIdx(0);
    const exact = names().find((n) => n.toLowerCase() === q.toLowerCase());
    if (exact) props.onSelect(exact);
  };

  // Click or Enter: select and clear so the box is blank (shows placeholder = selected name)
  const selectAndClear = (name: string) => {
    props.onSelect(name);
    close();
  };

  return (
    <div class={`relative ${props.class ?? ""}`}>
      <input
        ref={(el) => (input = el)}
        type="text"
        class="bg-neutral-800 border border-neutral-600 text-neutral-100 font-mono text-sm px-2 py-1 w-44"
        placeholder={props.placeholder ?? (props.value || "search layout…")}
        value={query()}
        onFocus={() => {
          // Clear on refocus and show the full list, ready to browse or type
          setQuery("");
          setResults(allSorted());
          setSelectedIdx(0);
          setOpen(true);
        }}
        onInput={(e) => {
          setQuery(e.currentTarget.value);
          doSearch(e.currentTarget.value);
        }}
        onBlur={() => {
          setTimeout(() => {
            if (!hovering()) close();
          }, 150);
        }}
        onKeyDown={(e) => {
          const res = results();
          if (e.key === "ArrowDown") {
            e.preventDefault();
            setSelectedIdx((i) => (i + 1) % Math.max(res.length, 1));
          } else if (e.key === "ArrowUp") {
            e.preventDefault();
            setSelectedIdx((i) => (i - 1 + Math.max(res.length, 1)) % Math.max(res.length, 1));
          } else if (e.key === "Enter") {
            e.preventDefault();
            if (res.length) selectAndClear(res[selectedIdx()]);
          } else if (e.key === "Escape") {
            close();
            input.blur();
          }
        }}
      />
      <Show when={open()}>
        <div
          class="absolute left-0 top-full mt-0.5 w-full bg-neutral-800 border border-neutral-600 z-50 flex flex-col max-h-64 overflow-y-auto"
          onMouseEnter={() => setHovering(true)}
          onMouseLeave={() => {
            setHovering(false);
            if (document.activeElement !== input) close();
          }}
        >
          <For
            each={results()}
            fallback={<div class="px-2 py-1 font-mono text-sm text-neutral-500">no matches</div>}
          >
            {(name, i) => (
              <div
                class="px-2 py-1 font-mono text-sm cursor-pointer"
                classList={{
                  "bg-neutral-600 text-neutral-100": i() === selectedIdx(),
                  "text-neutral-300 hover:bg-neutral-700": i() !== selectedIdx(),
                }}
                onMouseEnter={() => setSelectedIdx(i())}
                onClick={() => selectAndClear(name)}
              >
                {name}
              </div>
            )}
          </For>
        </div>
      </Show>
    </div>
  );
}
