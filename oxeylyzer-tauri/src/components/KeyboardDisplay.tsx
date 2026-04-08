import { For, Index } from "solid-js";

type Props = {
  keys: string;
  /** Optional: highlight these key characters */
  highlight?: string[];
};

const ROW_OFFSETS_REM = [0.25, 0, 0.75];

export default function KeyboardDisplay(props: Props) {
  const rows = () => [
    props.keys.slice(0, 10).split(""),
    props.keys.slice(10, 20).split(""),
    props.keys.slice(20, 30).split(""),
  ];

  const isHighlighted = (key: string) =>
    props.highlight ? props.highlight.includes(key) : false;

  return (
    <div class="inline-flex flex-col gap-[3px] font-mono select-none">
      <Index each={rows()}>
        {(row, rowIdx) => (
          <div
            class="flex gap-[3px]"
            style={{ "margin-left": `${ROW_OFFSETS_REM[rowIdx]}rem` }}
          >
            <For each={row()}>
              {(key, colIdx) => (
                <>
                  {colIdx() === 5 && <div class="w-3" />}
                  <div
                    class="w-7 h-7 border border-neutral-500 flex items-center justify-center text-xs"
                    classList={{
                      "border-white bg-neutral-700": isHighlighted(key),
                    }}
                  >
                    {key}
                  </div>
                </>
              )}
            </For>
          </div>
        )}
      </Index>
    </div>
  );
}
