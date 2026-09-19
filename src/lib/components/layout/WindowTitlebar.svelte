<script lang="ts">
  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import NotificationCenter from "$lib/components/layout/NotificationCenter.svelte";
  import Icon from "$lib/components/ui/Icon.svelte";

  let {
    notifications,
    onsettings,
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
    onsettings: () => void;
    ondismissnotification: (id: string) => void;
    onclearnotifications: () => void;
    onnotificationaction: (id: string) => void;
  } = $props();

  const appWindow = getCurrentWindow();
  let maximized = $state(false);

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

  const controlClass =
    "grid w-11 place-items-center rounded-lg p-2 bg-transparent z-0 text-[var(--color-text-muted)] hover:translate-y-0! hover:text-[var(--color-text-primary)] active:scale-100!";
</script>

<header
  class="flex h-11 shrink-0 items-stretch rounded-t-[12px] bg-[#0c0e12]/35 text-[var(--color-text-secondary)] shadow-[inset_0_-1px_0_rgba(255,255,255,.04)] backdrop-blur-2xl [.window-maximized_&]:rounded-none"
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

  <div class="flex" aria-label="Window controls">
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
      class="{controlClass} hover:text-white! "
      aria-label="Close window"
      title="Close"
    >
      <Icon name="close" size={15}></Icon>
    </button>
  </div>
</header>
