import { createSignal, onCleanup, onMount, Show } from "solid-js";
import { getCurrentWindow } from "@tauri-apps/api/window";

const MinimizeIcon = () => (
  <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24">
    <path fill="currentColor" d="M19 13H5v-2h14z" />
  </svg>
);

const MaximizeIcon = () => (
  <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24">
    <path fill="currentColor" d="M4 4h16v16H4zm2 4v10h12V8z" />
  </svg>
);

const RestoreIcon = () => (
  <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24">
    <path fill="currentColor" d="M6 8H4V4h12v2H8v12H6V8zm2 2h10v10H8V10z" />
  </svg>
);

const CloseIcon = () => (
  <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24">
    <path
      fill="currentColor"
      d="M13.46 12L19 17.54V19h-1.46L12 13.46L6.46 19H5v-1.46L10.54 12L5 6.46V5h1.46L12 10.54L17.54 5H19v1.46z"
    />
  </svg>
);

export default function TitleBar() {
  const win = getCurrentWindow();
  const [maximized, setMaximized] = createSignal(false);

  onMount(async () => {
    setMaximized(await win.isMaximized());

    const unlisten = await win.onResized(async () => {
      setMaximized(await win.isMaximized());
    });

    onCleanup(unlisten);
  });

  return (
    <div class="shrink-0 h-8 flex items-stretch bg-neutral-900 border-b border-neutral-700 select-none font-mono">
      {/* Drag region — fills all horizontal space left of the buttons */}
      <div class="flex-1 flex items-center px-3" data-tauri-drag-region>
        <span class="text-xs text-neutral-500" data-tauri-drag-region>
          Oxeylyzer
        </span>
      </div>

      {/* Window controls */}
      <div class="flex items-stretch">
        <button
          class="w-10 flex items-center justify-center text-neutral-400 hover:bg-neutral-700 hover:text-neutral-100"
          title="Minimize"
          onClick={() => win.minimize()}
        >
          <MinimizeIcon />
        </button>

        <button
          class="w-10 flex items-center justify-center text-neutral-400 hover:bg-neutral-700 hover:text-neutral-100"
          title={maximized() ? "Restore" : "Maximize"}
          onClick={() => win.toggleMaximize()}
        >
          <Show when={maximized()} fallback={<MaximizeIcon />}>
            <RestoreIcon />
          </Show>
        </button>

        <button
          class="w-10 flex items-center justify-center text-neutral-400 hover:bg-red-700 hover:text-white"
          title="Close"
          onClick={() => win.close()}
        >
          <CloseIcon />
        </button>
      </div>
    </div>
  );
}
