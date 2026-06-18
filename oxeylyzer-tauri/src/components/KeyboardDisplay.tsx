import { For, Show, createMemo, onCleanup } from "solid-js";
import {
  DragDropProvider,
  DragDropSensors,
  createDraggable,
  createDroppable,
  useDragDropContext,
  type DragEvent,
} from "@thisbeyond/solid-dnd";
import type { PhysKey } from "../types";
import { heatStyleFor } from "../heat";

/* eslint-disable no-unused-vars */
declare module "solid-js" {
  namespace JSX {
    interface Directives {
      draggable: ReturnType<typeof createDraggable>;
      droppable: ReturnType<typeof createDroppable>;
    }
  }
}
/* eslint-enable no-unused-vars */

const GAP = 0.3;

type Props = {
  keys: string;
  keyboard: PhysKey[];
  shape: number[];
  class?: string;
  highlight?: string[];
  heatmap?: Record<string, number>;
  /** Enable drag-and-drop swapping */
  draggable?: boolean;
  onSwap?: (_fromIdx: number, _toIdx: number) => void;
  /** Interactive click mode */
  interactive?: boolean;
  onKeyClick?: (_char: string, _idx: number) => void;
  /** Pinned key indices (GenerateView) */
  pinned?: Set<string>;
  /** Disabled key indices for analysis (AnalyzeView right-click) */
  disabledIndices?: Set<number>;
  onToggleDisabled?: (_idx: number) => void;
  /** Inline edit mode: which flat index is being edited */
  editingIdx?: number | null;
  onEditCommit?: (_idx: number, _char: string) => void;
  onEditNext?: (_idx: number) => void;
  onEditCancel?: () => void;
  onEditBackspace?: (_idx: number) => void;
  /** Per-key finger colors (flat array, same order as keys). Overrides heatmap. */
  fingerColors?: string[];
};

interface KeyTileProps {
  char: string;
  flatIdx: number;
  isHighlighted: boolean;
  isPinned: boolean;
  isDisabled: boolean;
  heatStyle: string;
  fingerColor?: string;
  interactive: boolean;
  draggableEnabled: boolean;
  editingIdx: number | null | undefined;
  onContextMenu: (_idx: number) => void;
  onClick: (_char: string, _idx: number) => void;
  onEditCommit: (_idx: number, _char: string) => void;
  onEditNext: (_idx: number) => void;
  onEditCancel: () => void;
  onEditBackspace: () => void;
}

const KeyTile = (props: KeyTileProps) => {
  const draggable = createDraggable(props.flatIdx);
  const droppable = createDroppable(props.flatIdx);

  let longPressTimer: ReturnType<typeof setTimeout> | null = null;
  let longPressFired = false;

  const cancelLongPress = () => {
    if (longPressTimer !== null) {
      clearTimeout(longPressTimer);
      longPressTimer = null;
    }
  };

  onCleanup(cancelLongPress);

  const isEditing = () => props.editingIdx === props.flatIdx;

  const baseClass =
    "w-full h-full rounded-[12%] flex items-center justify-center select-none touch-none relative";

  const effectiveStyle = () => {
    if (props.isHighlighted) return "";
    if (props.fingerColor) return props.fingerColor;
    return props.heatStyle;
  };

  return (
    <div
      use:draggable={draggable}
      use:droppable={droppable}
      class={baseClass}
      classList={{
        "border-white bg-neutral-600": props.isHighlighted,
        "border-neutral-400 ring-1 ring-yellow-400/50": props.isPinned,
        "border-neutral-500 opacity-30": props.isDisabled,
        "border-neutral-500": !props.isHighlighted && !props.isPinned && !props.isDisabled,
        "cursor-pointer hover:border-neutral-300":
          (props.interactive || props.draggableEnabled) && !isEditing(),
      }}
      style={effectiveStyle()}
      onContextMenu={(e) => {
        e.preventDefault();
        props.onContextMenu(props.flatIdx);
      }}
      onClick={() => props.onClick(props.char, props.flatIdx)}
      onTouchStart={() => {
        longPressFired = false;
        cancelLongPress();
        longPressTimer = setTimeout(() => {
          longPressTimer = null;
          longPressFired = true;
          props.onContextMenu(props.flatIdx);
        }, 500);
      }}
      onTouchMove={cancelLongPress}
      onTouchEnd={(e) => {
        cancelLongPress();
        if (longPressFired) {
          e.preventDefault();
          longPressFired = false;
        }
      }}
    >
      {props.char}
      {props.isPinned && (
        <span class="absolute top-0 right-0 text-[9px] text-yellow-300 leading-none p-px">⚑</span>
      )}
      <Show when={isEditing()}>
        {/* Pulsating ring indicates edit mode */}
        <div class="absolute inset-0 rounded-[12%] ring-2 ring-white/60 animate-pulse pointer-events-none" />
        {/* Invisible input captures keystrokes; char text shows beneath */}
        <input
          class="absolute inset-0 w-full h-full bg-transparent outline-none text-transparent cursor-default"
          style={{ "caret-color": "transparent" }}
          ref={(el) => setTimeout(() => el?.focus(), 0)}
          onInput={(e) => {
            const val = e.currentTarget.value;
            if (val) {
              props.onEditCommit(props.flatIdx, val[val.length - 1]);
              e.currentTarget.value = "";
            }
          }}
          onKeyDown={(e) => {
            if (e.key === "Enter" || e.key === "Tab") {
              e.preventDefault();
              props.onEditNext(props.flatIdx);
            } else if (e.key === "Escape") {
              props.onEditCancel();
            } else if (e.key === "Backspace" || e.key === "Delete") {
              e.preventDefault();
              props.onEditBackspace();
            }
          }}
        />
      </Show>
    </div>
  );
};

export default function KeyboardDisplay(props: Props) {
  const geom = createMemo(() => {
    const kb = props.keyboard;
    if (!kb || kb.length === 0) return null;
    let minX = Infinity,
      minY = Infinity,
      maxX = -Infinity,
      maxY = -Infinity;
    for (const [x, y, w, h] of kb) {
      minX = Math.min(minX, x);
      minY = Math.min(minY, y);
      maxX = Math.max(maxX, x + w);
      maxY = Math.max(maxY, y + h);
    }
    const dx = maxX - minX,
      dy = maxY - minY;
    const kw = 100 / dx;
    const ym = dx / dy;
    return { kw, ym, heightCss: dy * kw, fontSizeCqw: kw / 2.25, minX, minY };
  });

  const chars = () => props.keys.split("");

  const isHighlighted = (key: string) => props.highlight?.includes(key) ?? false;
  const isPinned = (key: string) => props.pinned?.has(key) ?? false;

  const heatStyle = (key: string): string => {
    if (!props.heatmap) return "";
    return heatStyleFor(props.heatmap[key] ?? 0);
  };

  const handleDragEnd = ({ draggable, droppable }: DragEvent) => {
    if (!droppable || draggable.id === droppable.id) return;
    props.onSwap?.(draggable.id as number, droppable.id as number);
  };

  const inner = () => {
    const g = geom();
    if (!g) return null;
    const { kw, ym, heightCss, fontSizeCqw, minX, minY } = g;

    return (
      <div
        class="relative w-full"
        style={{
          "aspect-ratio": `100 / ${heightCss}`,
          "font-size": `${fontSizeCqw.toFixed(2)}cqw`,
          "line-height": "0",
        }}
      >
        <For each={chars()}>
          {(char, i) => {
            const flatIdx = i();
            const pk = props.keyboard[flatIdx];
            if (!pk) return null;
            const [px, py, pw, ph] = pk;
            const dndCtx = useDragDropContext();
            const isActiveDrag = () =>
              dndCtx ? dndCtx[0].active.draggable?.id === flatIdx : false;

            return (
              <div
                class="absolute"
                style={{
                  left: `${(px - minX) * kw + GAP}%`,
                  top: `${(py - minY) * kw * ym + GAP * ym}%`,
                  width: `${pw * kw - GAP * 2}%`,
                  height: `${(ph * kw - GAP * 2) * ym}%`,
                  opacity: isActiveDrag() ? "0.7" : "1",
                  "z-index": isActiveDrag() ? "10" : "auto",
                }}
              >
                <KeyTile
                  char={char}
                  flatIdx={flatIdx}
                  isHighlighted={isHighlighted(char)}
                  isPinned={isPinned(char)}
                  isDisabled={props.disabledIndices?.has(flatIdx) ?? false}
                  heatStyle={heatStyle(char)}
                  fingerColor={props.fingerColors?.[flatIdx]}
                  interactive={props.interactive ?? false}
                  draggableEnabled={props.draggable ?? false}
                  editingIdx={props.editingIdx}
                  onContextMenu={(idx) => props.onToggleDisabled?.(idx)}
                  onClick={(ch, idx) => props.onKeyClick?.(ch, idx)}
                  onEditCommit={(idx, ch) => props.onEditCommit?.(idx, ch)}
                  onEditNext={(idx) => props.onEditNext?.(idx)}
                  onEditCancel={() => props.onEditCancel?.()}
                  onEditBackspace={() => props.onEditBackspace?.(flatIdx)}
                />
              </div>
            );
          }}
        </For>
      </div>
    );
  };

  return (
    <Show when={geom()}>
      <div
        class={`w-full overflow-hidden ${props.class ?? ""}`}
        style={{ "container-type": "inline-size" }}
      >
        <DragDropProvider onDragEnd={handleDragEnd}>
          {/* Without sensors, drags can never start — read-only boards
              (layout lists, results) shouldn't have pick-up-able keys. */}
          <Show when={props.draggable} fallback={inner()}>
            <DragDropSensors>{inner()}</DragDropSensors>
          </Show>
        </DragDropProvider>
      </div>
    </Show>
  );
}
