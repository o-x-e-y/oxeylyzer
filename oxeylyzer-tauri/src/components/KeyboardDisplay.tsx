import { For, Index } from "solid-js";

type Props = {
    keys: string;
    /** Optional: highlight these key characters */
    highlight?: string[];
    /** Optional: char → frequency percent (0–100), drives heat coloring */
    heatmap?: Record<string, number>;
    /** When true, keys are clickable and call onKeyClick */
    interactive?: boolean;
    /** Chars that are currently pinned (shown with lock indicator) */
    pinned?: Set<string>;
    /** Called when a key is clicked in interactive mode */
    onKeyClick?: (char: string) => void;
};

const ROW_OFFSETS_REM = [0.25, 0, 0.75];

/** Returns an rgb() string on a red heat scale. 0% → cool grey, 100% → deep red. */
function heatColor(percent: number): string {
    // Match the repl's formula: complement = 215 - (freq/total) * 1720, clamped [0, 215]
    const complement = Math.max(0, Math.min(215, 215 - (percent / 100) * 1720));
    const c = Math.round(complement);
    return `rgb(215, ${c}, ${c})`;
}

export default function KeyboardDisplay(props: Props) {
    const rows = () => [
        props.keys.slice(0, 10).split(""),
        props.keys.slice(10, 20).split(""),
        props.keys.slice(20, 30).split(""),
    ];

    const isHighlighted = (key: string) =>
        props.highlight ? props.highlight.includes(key) : false;

    const isPinned = (key: string) => props.pinned?.has(key) ?? false;

    const keyBg = (key: string): string => {
        if (isHighlighted(key)) return "";
        if (!props.heatmap) return "";
        const pct = props.heatmap[key] ?? 0;
        if (pct === 0) return "";
        return `background-color: ${heatColor(pct)}`;
    };

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
                                        class="w-7 h-7 border flex items-center justify-center text-xs relative"
                                        classList={{
                                            "border-white bg-neutral-700": isHighlighted(key),
                                            "border-neutral-400 ring-1 ring-neutral-400": isPinned(key),
                                            "border-neutral-500": !isHighlighted(key) && !isPinned(key),
                                            "cursor-pointer hover:border-neutral-300": !!props.interactive,
                                        }}
                                        style={keyBg(key)}
                                        onClick={() => props.interactive && props.onKeyClick?.(key)}
                                    >
                                        {key}
                                        {isPinned(key) && (
                                            <span class="absolute top-0 right-0 text-[7px] text-neutral-300 leading-none p-px">
                                                ⚑
                                            </span>
                                        )}
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
