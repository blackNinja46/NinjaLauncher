<script lang="ts">
  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";

  const appWindow = getCurrentWindow();
  let maximized = $state(false);

  async function refreshMaximized() {
    maximized = await appWindow.isMaximized().catch(() => false);
  }

  async function minimize() {
    await appWindow.minimize();
  }

  async function toggleMaximize() {
    await appWindow.toggleMaximize();
    await refreshMaximized();
  }

  async function close() {
    await appWindow.close();
  }

  onMount(() => {
    void refreshMaximized();
    let unlisten: (() => void) | undefined;
    appWindow.onResized(refreshMaximized).then((callback) => (unlisten = callback));
    return () => unlisten?.();
  });
</script>

<header
  class="window-titlebar flex h-11 shrink-0 items-stretch border-b border-cyan-300/20 bg-[#020a1b] text-[#b9d8f2]"
>
  <div
    data-tauri-drag-region
    role="presentation"
    ondblclick={toggleMaximize}
    class="flex min-w-0 flex-1 select-none items-center gap-2.5 px-3"
  >
    <img
      data-tauri-drag-region
      src="/brand/ninja-launcher-mark.png"
      alt=""
      class="pointer-events-none h-7 w-8 object-contain"
    />
    <span
      data-tauri-drag-region
      class="pointer-events-none truncate text-[11px] font-extrabold uppercase tracking-[.11em]"
    >
      Ninja Launcher
    </span>
    <span
      data-tauri-drag-region
      class="pointer-events-none text-[9px] font-bold uppercase tracking-[.14em] text-cyan-300/40"
    >
      Java Edition
    </span>
  </div>

  <div class="flex" aria-label="Window controls">
    <button
      type="button"
      onclick={minimize}
      class="window-control"
      aria-label="Minimize window"
      title="Minimize"
    >
      <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M3 8.5h10" /></svg>
    </button>
    <button
      type="button"
      onclick={toggleMaximize}
      class="window-control"
      aria-label={maximized ? "Restore window" : "Maximize window"}
      title={maximized ? "Restore" : "Maximize"}
    >
      {#if maximized}
        <svg viewBox="0 0 16 16" aria-hidden="true">
          <path d="M5.5 5.5h7v7h-7zM3.5 10.5h-1v-7h7v1" />
        </svg>
      {:else}
        <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M3.5 3.5h9v9h-9z" /></svg>
      {/if}
    </button>
    <button
      type="button"
      onclick={close}
      class="window-control window-close"
      aria-label="Close window"
      title="Close"
    >
      <svg viewBox="0 0 16 16" aria-hidden="true"><path d="m4 4 8 8m0-8-8 8" /></svg>
    </button>
  </div>
</header>

<style>
  .window-titlebar {
    box-shadow:
      inset 0 1px 0 rgb(63 213 255 / 12%),
      inset 0 0 0 1px rgb(18 116 201 / 24%),
      0 5px 18px rgb(0 4 18 / 35%);
  }

  .window-control {
    display: grid;
    width: 46px;
    place-items: center;
    border: 0;
    border-left: 1px solid rgb(74 195 245 / 8%);
    background: transparent;
    color: #87a9c7;
    transition:
      color 120ms ease,
      background 120ms ease;
  }

  .window-control:hover {
    background: rgb(13 119 191 / 28%);
    color: #eaf8ff;
  }

  .window-control:active {
    background: rgb(8 91 158 / 45%);
  }

  .window-control svg {
    width: 15px;
    height: 15px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.25;
    shape-rendering: crispEdges;
  }

  .window-close:hover {
    background: #c42b3b;
    color: white;
  }

  .window-close:active {
    background: #921f2d;
  }
</style>
