<script lang="ts">
  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import NotificationCenter from "$lib/components/layout/NotificationCenter.svelte";
  import Icon from "$lib/components/ui/Icon.svelte";
  import type { LauncherInstance } from "$lib/types";

  let {
    notifications,
    runningInstances,
    stoppingInstanceIds,
    onsettings,
    onstopinstance,
    onopenconsole,
    ondismissnotification,
    onclearnotifications,
    onnotificationaction,
  }: {
    notifications: {
      id: string;
      title: string;
      message: string;
      tone: "info" | "success" | "warning" | "error";
      actionLabel?: string;
    }[];
    runningInstances: LauncherInstance[];
    stoppingInstanceIds: string[];
    onsettings: () => void;
    onstopinstance: (id: string) => void | Promise<void>;
    onopenconsole: (id: string) => void;
    ondismissnotification: (id: string) => void;
    onclearnotifications: () => void;
    onnotificationaction: (id: string) => void;
  } = $props();

  const appWindow = getCurrentWindow();
  let maximized = $state(false);
  let runningMenuOpen = $state(false);
  let runningMenuElement = $state<HTMLDivElement>();
  let runningMenuPanel = $state<HTMLDivElement>();

  async function refreshMaximized() {
    maximized = await appWindow.isMaximized().catch(() => false);
    document.documentElement.classList.toggle("window-maximized", maximized);
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
    return () => {
      document.documentElement.classList.remove("window-maximized");
      unlisten?.();
    };
  });

  $effect(() => {
    if (!runningMenuOpen) return;
    const closeOutside = (event: PointerEvent) => {
      const target = event.target as Node;
      if (!runningMenuElement?.contains(target) && !runningMenuPanel?.contains(target)) {
        runningMenuOpen = false;
      }
    };
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") runningMenuOpen = false;
    };
    document.addEventListener("pointerdown", closeOutside);
    document.addEventListener("keydown", closeOnEscape);
    return () => {
      document.removeEventListener("pointerdown", closeOutside);
      document.removeEventListener("keydown", closeOnEscape);
    };
  });

  function portalRunningMenu(node: HTMLElement) {
    document.body.appendChild(node);

    const position = () => {
      const bounds = runningMenuElement?.getBoundingClientRect();
      if (!bounds) return;
      const width = Math.min(340, window.innerWidth - 24);
      node.style.width = `${width}px`;
      node.style.left = `${Math.max(12, Math.min(bounds.right - width, window.innerWidth - width - 12))}px`;
      node.style.top = `${bounds.bottom + 7}px`;
    };

    position();
    window.addEventListener("resize", position);
    return {
      destroy() {
        window.removeEventListener("resize", position);
        node.remove();
      },
    };
  }

  const controlClass =
    "grid w-11 place-items-center rounded-lg p-2 bg-transparent z-0 text-[var(--color-text-muted)] hover:translate-y-0! hover:text-[var(--color-text-primary)] active:scale-100!";
</script>

<header
  class="relative z-[100] flex h-11 shrink-0 items-stretch rounded-t-[12px] bg-[#0c0e12]/35 text-[var(--color-text-secondary)] shadow-[inset_0_-1px_0_rgba(255,255,255,.04)] backdrop-blur-2xl [.window-maximized_&]:rounded-none"
>
  <div
    data-tauri-drag-region
    role="presentation"
    ondblclick={toggleMaximize}
    class="flex min-w-0 flex-1 select-none items-center px-4"
  >
    <span
      data-tauri-drag-region
      class="pointer-events-none truncate text-[14px] font-bold tracking-[-.01em]"
    >
      Ninja Launcher
    </span>
  </div>

  <div class="flex items-stretch" aria-label="Window controls">
    <div bind:this={runningMenuElement} class="relative flex items-center pr-1">
      <button
        type="button"
        aria-label="Running Minecraft instances"
        aria-haspopup="menu"
        aria-expanded={runningMenuOpen}
        onclick={() => (runningMenuOpen = !runningMenuOpen)}
        class="mx-1 flex h-8 items-center gap-2 rounded-[7px] border border-white/[.08] bg-white/[.035] px-3 text-[11px] font-semibold text-[var(--color-text-secondary)] transition hover:border-white/[.15] hover:bg-white/[.07] hover:text-white"
        title="Running instances"
      >
        <span
          class="size-1.5 rounded-full {runningInstances.length
            ? 'bg-emerald-400 shadow-[0_0_8px_rgba(52,211,153,.85)]'
            : 'bg-slate-500'}"
        ></span>
        <Icon name="logs" size={13} />
        <span>{runningInstances.length} running</span>
        <span class="transition-transform {runningMenuOpen ? 'rotate-180' : ''}">
          <Icon name="chevron-down" size={10} />
        </span>
      </button>

      {#if runningMenuOpen}
        <div
          bind:this={runningMenuPanel}
          use:portalRunningMenu
          role="menu"
          aria-label="Running Minecraft instances"
          class="running-instances-menu fixed z-[140] overflow-hidden rounded-[10px] border border-white/[.11] shadow-[0_22px_65px_rgba(0,4,18,.72),inset_0_1px_0_rgba(255,255,255,.07)]"
        >
          <div
            class="flex items-center justify-between border-b border-white/[.08] bg-black/[.08] px-4 py-3"
          >
            <div>
              <div class="text-xs font-bold text-white">Running instances</div>
              <div class="mt-0.5 text-[10px] text-[var(--color-text-faint)]">
                Open a console or stop a game directly.
              </div>
            </div>
            <span
              class="rounded-full bg-emerald-400/10 px-2 py-1 text-[9px] font-bold text-emerald-300"
            >
              {runningInstances.length} ACTIVE
            </span>
          </div>

          {#if runningInstances.length}
            <div class="max-h-72 overflow-y-auto p-2">
              {#each runningInstances as item (item.id)}
                <div
                  class="group flex items-center gap-3 rounded-[7px] border border-transparent px-3 py-2.5 transition hover:border-white/[.08] hover:bg-white/[.045]"
                >
                  <span
                    class="size-2 shrink-0 rounded-full bg-emerald-400 shadow-[0_0_7px_rgba(52,211,153,.7)]"
                  ></span>
                  <div class="min-w-0 flex-1">
                    <div class="truncate text-xs font-bold text-white">{item.name}</div>
                    <div class="mt-0.5 truncate text-[10px] text-[var(--color-text-faint)]">
                      Minecraft {item.minecraftVersion} · {item.loader === "vanilla"
                        ? "Vanilla"
                        : "Fabric"}
                    </div>
                  </div>
                  <button
                    type="button"
                    class="grid size-8 shrink-0 place-items-center rounded-[6px] border border-white/[.08] bg-white/[.035] text-[var(--color-text-muted)] transition hover:border-[var(--color-primary)]/35 hover:bg-[var(--color-primary)]/10 hover:text-white"
                    aria-label={`Open ${item.name} console`}
                    title="Open console"
                    onclick={() => {
                      onopenconsole(item.id);
                      runningMenuOpen = false;
                    }}
                  >
                    <Icon name="logs" size={13} />
                  </button>
                  <button
                    type="button"
                    disabled={stoppingInstanceIds.includes(item.id)}
                    class="grid size-8 shrink-0 place-items-center rounded-[6px] border border-red-400/15 bg-red-400/[.055] text-red-300 transition hover:border-red-400/35 hover:bg-red-400/10 hover:text-red-200 disabled:cursor-wait disabled:opacity-45"
                    aria-label={`Stop ${item.name}`}
                    title={stoppingInstanceIds.includes(item.id) ? "Stopping…" : "Stop instance"}
                    onclick={() => void onstopinstance(item.id)}
                  >
                    <Icon name="stop" size={12} />
                  </button>
                </div>
              {/each}
            </div>
          {:else}
            <div class="px-5 py-8 text-center">
              <div class="text-xs font-semibold text-[var(--color-text-secondary)]">
                No Minecraft instance is running.
              </div>
              <div class="mt-1 text-[10px] text-[var(--color-text-faint)]">
                Started profiles will appear here.
              </div>
            </div>
          {/if}
        </div>
      {/if}
    </div>
    <NotificationCenter
      items={notifications}
      embedded
      ondismiss={ondismissnotification}
      onclear={onclearnotifications}
      onaction={onnotificationaction}
    />
    <button
      type="button"
      onclick={onsettings}
      class={controlClass}
      aria-label="Open settings"
      title="Settings"
    >
      <Icon name="settings" size={15} />
    </button>
    <span class="my-2 w-px bg-white/[.07]" aria-hidden="true"></span>
    <button
      type="button"
      onclick={minimize}
      class={controlClass}
      aria-label="Minimize window"
      title="Minimize"
    >
      <Icon name="minus" size={15}></Icon>
    </button>
    <button
      type="button"
      onclick={toggleMaximize}
      class={controlClass}
      aria-label={maximized ? "Restore window" : "Maximize window"}
      title={maximized ? "Restore" : "Maximize"}
    >
      {#if maximized}
        <Icon name="downLeftAndUpRightToCenter" size={15}></Icon>
      {:else}
        <Icon name="upRightAndDownLeftFromCenter" size={15}></Icon>
      {/if}
    </button>
    <button
      type="button"
      onclick={close}
      class="{controlClass} hover:text-white!"
      aria-label="Close window"
      title="Close"
    >
      <Icon name="close" size={15}></Icon>
    </button>
  </div>
</header>

<style>
  .running-instances-menu {
    background: color-mix(in srgb, var(--color-navy-900) 66%, transparent);
    -webkit-backdrop-filter: blur(28px) saturate(155%);
    backdrop-filter: blur(28px) saturate(155%);
  }
</style>
