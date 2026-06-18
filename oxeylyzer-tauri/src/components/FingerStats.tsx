import { For } from "solid-js";
import type { LayoutStats } from "../types";

const FINGER_LABELS = ["LP", "LR", "LM", "LI", "LT", "RT", "RI", "RM", "RR", "RP"] as const;

/**
 * Per-finger usage bars with finger-speed underline, plus hand balance.
 * Usage = % of all keystrokes typed by that finger; speed = the analyzer's
 * per-finger fspeed penalty (higher = that finger does more same-finger work).
 */
export default function FingerStats(props: { stats: LayoutStats }) {
  const usage = () => props.stats.finger_usage ?? [];
  const speed = () => props.stats.finger_speed ?? [];
  const maxUsage = () => Math.max(...usage(), 1e-9);
  const maxSpeed = () => Math.max(...speed(), 1e-9);
  const hand = (from: number, to: number) =>
    usage()
      .slice(from, to)
      .reduce((a, b) => a + b, 0);

  return (
    <div class="flex flex-col gap-2 border border-neutral-700 p-3 w-fit">
      <div class="text-xs text-neutral-500 uppercase tracking-widest">Finger Load</div>

      <div class="flex items-end gap-1">
        <For each={FINGER_LABELS as unknown as string[]}>
          {(label, i) => (
            <div
              class="flex flex-col items-center gap-1 w-9 font-mono"
              classList={{ "mr-3": i() === 4 }}
              title={`${label} — usage ${usage()[i()]?.toFixed(2)}%, speed ${speed()[i()]?.toFixed(3)}`}
            >
              <span class="text-[10px] text-neutral-400">{usage()[i()]?.toFixed(1)}</span>
              <div class="h-20 w-3.5 bg-neutral-800 border border-neutral-700 flex flex-col justify-end">
                <div
                  class="bg-neutral-400 w-full"
                  style={{ height: `${((usage()[i()] ?? 0) / maxUsage()) * 100}%` }}
                />
              </div>
              {/* finger speed share as a thin underline */}
              <div class="h-1 w-3.5 bg-neutral-800">
                <div
                  class="bg-red-400/70 h-full"
                  style={{ width: `${((speed()[i()] ?? 0) / maxSpeed()) * 100}%` }}
                />
              </div>
              <span class="text-[10px] text-neutral-500">{label}</span>
            </div>
          )}
        </For>
      </div>

      <div class="flex gap-4 font-mono text-xs text-neutral-400">
        <span>
          left <span class="text-neutral-200">{hand(0, 5).toFixed(1)}%</span>
        </span>
        <span>
          right <span class="text-neutral-200">{hand(5, 10).toFixed(1)}%</span>
        </span>
        <span class="text-neutral-600">bar = usage · red line = finger speed</span>
      </div>
    </div>
  );
}
