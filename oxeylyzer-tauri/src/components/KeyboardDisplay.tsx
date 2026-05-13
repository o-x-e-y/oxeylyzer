import { For, Show, createMemo, onCleanup } from "solid-js";
import {
  DragDropProvider,
  DragDropSensors,
  createDraggable,
  createDroppable,
  useDragDropContext,
} from "@thisbeyond/solid-dnd";
import type { PhysKey } from "../mock";
import { heatScheme } from "../store";

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
};

function heatColorOriginal(pct: number): string {
  const c = Math.max(0, Math.min(215, 215 - (pct / 100) * 1720));
  return `background-color:rgb(215,${Math.round(c)},${Math.round(c)});color:#111`;
}

function heatColorPlayground(pct: number): string {
  const p = pct / 100;
  const v = p * 30 + Math.log(p * 120 + 1);
  const b = 95;
  return `background-color:rgb(${Math.round(b * 0.9 + v * 18)},${Math.round(b * 1.3 - v * 10)},${Math.round(b * 1.325 - v * 10)})`;
}

function heatColorV2(pct: number): string {
  const f = Math.min(pct, 14) / 14;
  return `background-color:rgb(${Math.round(140 + f * 115)},${Math.round(140 - f * 140)},${Math.round(140 - f * 140)});color:#111`;
}

interface KeyTileProps {
  char: string;
  flatIdx: number;
  isHighlighted: boolean;
  isPinned: boolean;
  isDisabled: boolean;
  heatStyle: string;
  interactive: boolean;
  draggableEnabled: boolean;
  editingIdx: number | null | undefined;
  onContextMenu: (_idx: number) => void;
  onClick: (_char: string, _idx: number) => void;
  onEditCommit: (_idx: number, _char: string) => void;
  onEditNext: (_idx: number) => void;
  onEditCancel: () => void;
}

const KeyTile = (props: KeyTileProps) => {
  // eslint-disable-next-line no-unused-vars
  const draggable = createDraggable(props.flatIdx);
  // eslint-disable-next-line no-unused-vars
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

  return (
    <div
      use:draggable
      use:droppable
      class={baseClass}
      classList={{
        "border-white bg-neutral-600": props.isHighlighted,
        "border-neutral-400 ring-1 ring-yellow-400/50": props.isPinned,
        "border-neutral-500 opacity-30": props.isDisabled,
        "border-neutral-500": !props.isHighlighted && !props.isPinned && !props.isDisabled,
        "cursor-pointer hover:border-neutral-300": props.interactive || props.draggableEnabled,
      }}
      style={props.isHighlighted ? "" : props.heatStyle}
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
      <Show
        when={isEditing()}
        fallback={
          <>
            {props.char}
            {props.isPinned && (
              <span class="absolute top-0 right-0 text-[9px] text-yellow-300 leading-none p-px">
                ⚑
              </span>
            )}
          </>
        }
      >
        <input
          class="w-full h-full text-center bg-transparent outline-none font-mono text-[1em] caret-neutral-300"
          maxLength={1}
          ref={(el) => setTimeout(() => el?.focus(), 0)}
          onInput={(e) => {
            const val = e.currentTarget.value;
            if (val) props.onEditCommit(props.flatIdx, val);
          }}
          onKeyDown={(e) => {
            if (e.key === "Enter" || e.key === "Tab") {
              e.preventDefault();
              props.onEditNext(props.flatIdx);
            } else if (e.key === "Escape") {
              props.onEditCancel();
            } else if (e.key === "Backspace" || e.key === "Delete") {
              e.preventDefault();
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
    const pct = props.heatmap[key] ?? 0;
    switch (heatScheme()) {
      case "original":
        return heatColorOriginal(pct);
      case "v2":
        return heatColorV2(pct);
      default:
        return heatColorPlayground(pct);
    }
  };

  const handleDragEnd = ({
    draggable,
    droppable,
  }: {
    draggable: { id: number };
    droppable: { id: number } | null;
  }) => {
    if (!droppable || draggable.id === droppable.id) return;
    props.onSwap?.(draggable.id, droppable.id);
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
                  interactive={props.interactive ?? false}
                  draggableEnabled={props.draggable ?? false}
                  editingIdx={props.editingIdx}
                  onContextMenu={(idx) => props.onToggleDisabled?.(idx)}
                  onClick={(ch, idx) => props.onKeyClick?.(ch, idx)}
                  onEditCommit={(idx, ch) => props.onEditCommit?.(idx, ch)}
                  onEditNext={(idx) => props.onEditNext?.(idx)}
                  onEditCancel={() => props.onEditCancel?.()}
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
          <DragDropSensors>{inner()}</DragDropSensors>
        </DragDropProvider>
      </div>
    </Show>
  );
}
