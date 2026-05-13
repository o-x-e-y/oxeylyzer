import type { JSX } from "solid-js";

type Props = {
  value?: string;
  onChange: (_value: string) => void;
  class?: string;
  children: JSX.Element;
};

export default function Dropdown(props: Props) {
  return (
    <div class={`relative ${props.class ?? ""}`}>
      <select
        class="appearance-none bg-neutral-800 border border-neutral-600 text-neutral-100 font-mono text-sm px-2 py-1 pr-6 w-full"
        value={props.value}
        onChange={(e) => props.onChange(e.currentTarget.value)}
      >
        {props.children}
      </select>
      <span class="pointer-events-none absolute right-1.5 top-1/2 -translate-y-1/2 text-neutral-400 text-[10px]">
        ▼
      </span>
    </div>
  );
}
