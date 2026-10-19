<script lang="ts">
  import { onMount } from "svelte";
  import { fade, fly } from "svelte/transition";
  import Icon from "$lib/components/ui/Icon.svelte";
  import InlineMessage from "$lib/components/ui/InlineMessage.svelte";

  type NotificationTone = "info" | "success" | "warning" | "error";
  type NotificationItem = {
    id: string;
    title: string;
    message: string;
    tone: NotificationTone;
    actionLabel?: string;
  };

  let {
    items,
    embedded = false,
    ondismiss,
    onclear,
    onaction,
  }: {
    items: NotificationItem[];
    embedded?: boolean;
    ondismiss: (id: string) => void;
    onclear: () => void;
    onaction: (id: string) => void;
  } = $props();

  let open = $state(false);
  let container = $state<HTMLDivElement>();
  let menu = $state<HTMLElement>();

  const toneClasses: Record<NotificationTone, string> = {
    info: "bg-[var(--color-primary)] shadow-[0_0_10px_rgba(59,157,245,.55)]",
    success: "bg-[var(--color-success)]",
    warning: "bg-[var(--color-warning)]",
    error: "bg-[var(--color-danger)]",
  };

  $effect(() => {
    if (items.length === 0) open = false;
  });

  $effect(() => {
    if (!menu) return;
    if (open && !menu.matches(":popover-open")) menu.showPopover();
    if (!open && menu.matches(":popover-open")) menu.hidePopover();
  });

  onMount(() => {
    function closeOnOutsideClick(event: PointerEvent) {
      if (open && container && !container.contains(event.target as Node)) open = false;
    }

    document.addEventListener("pointerdown", closeOnOutsideClick);
    return () => document.removeEventListener("pointerdown", closeOnOutsideClick);
  });
</script>

<div bind:this={container} class={embedded ? "relative z-30 h-full" : "fixed right-4 top-3.5 z-30"}>
  <button
    type="button"
    class={embedded
      ? `relative grid h-full w-11 place-items-center rounded-lg border-0 bg-transparent text-[var(--color-text-muted)] hover:translate-y-0! hover:text-[var(--color-text-primary)] active:scale-100! ${items.length ? "text-[var(--color-text-primary)]" : ""}`
      : `relative grid size-9 place-items-center text-[var(--color-text-muted)] ${items.length ? "border-[var(--color-border-strong)] text-[var(--color-text-primary)]" : "border-[var(--color-border-default)] hover:border-[var(--color-border-strong)] hover:text-[var(--color-text-primary)]"}`}
    aria-label={`Notifications${items.length ? `, ${items.length} unread` : ""}`}
    aria-expanded={open}
    title="Notifications"
    onclick={() => (open = !open)}
  >
    <Icon name="bell" size={16} />
    {#if items.length > 0}<span
        class="absolute grid h-[17px] min-w-[17px] place-items-center rounded-full border-2 border-[#0c0e12] bg-[var(--color-primary)] text-[9px] font-extrabold text-white {embedded
          ? 'right-1 top-0.5'
          : '-right-[5px] -top-[5px]'}">{items.length}</span
      >{/if}
  </button>

  {#if open}
    <section
      bind:this={menu}
      popover="manual"
      class="fixed bottom-auto left-auto right-4 top-14 z-[200] m-0 w-[min(320px,calc(100vw-32px))] overflow-hidden rounded-[10px] border-0 bg-[#0d0f14]/10 p-0 text-inherit shadow-[0_24px_70px_rgba(0,0,0,.46),inset_0_1px_0_rgba(255,255,255,.07)] backdrop-blur-md"
      aria-label="Notifications"
      transition:fly={{ y: -8, duration: 180 }}
    >
      <header
        class="flex items-center justify-between border-b border-[var(--color-border-subtle)] px-[15px] py-3.5"
      >
        <div>
          <h2 class="m-0 text-[22px] font-bold">Notifications</h2>
          <p class="mb-0 mt-0.5 text-[10px] text-[var(--color-text-faint)]">
            {items.length}
            {items.length === 1 ? "message" : "messages"}
          </p>
        </div>
        <button
          type="button"
          class="grid size-7 place-items-center border-0 bg-transparent text-[var(--color-text-faint)] hover:bg-white/[.07] hover:text-[var(--color-text-primary)]"
          aria-label="Close notifications"
          onclick={() => (open = false)}
        >
          <Icon name="close" size={14} />
        </button>
      </header>

      <div class="h-85 max-h-[min(430px,calc(100vh-125px))] overflow-y-auto p-[7px]">
        {#each items as item (item.id)}
          <article
            class="relative flex gap-[11px] rounded-[7px] border-b border-[var(--color-border-subtle)] py-3 pl-3 pr-[9px] last:border-b-0"
            in:fly={{ x: 10, duration: 180 }}
            out:fade={{ duration: 120 }}
          >
            <span
              class="mt-[5px] size-[7px] shrink-0 rounded-full {toneClasses[item.tone]}"
              aria-hidden="true"
            ></span>
            <div class="min-w-0 flex-1">
              <h3 class="m-0 text-xs font-bold text-[var(--color-text-primary)]">{item.title}</h3>
              <p class="mb-0 mt-1 text-[11px] leading-[1.55] text-[var(--color-text-muted)]">
                <InlineMessage message={item.message} />
              </p>
              {#if item.actionLabel}
                <button
                  class="mt-[9px] rounded-[5px] border border-[var(--color-border-default)] bg-[var(--color-surface-subtle)] px-[9px] py-[5px] text-[10px] font-bold text-[var(--color-text-secondary)] hover:border-[var(--color-border-brand)] hover:bg-[var(--color-surface-hover)] hover:text-white"
                  onclick={() => onaction(item.id)}
                >
                  {item.actionLabel}
                </button>
              {/if}
            </div>
            <button
              type="button"
              class="grid size-6 shrink-0 place-items-center border-0 bg-transparent text-[var(--color-text-faint)] hover:bg-white/[.07] hover:text-[var(--color-text-primary)]"
              aria-label={`Dismiss ${item.title}`}
              onclick={() => ondismiss(item.id)}
            >
              <Icon name="close" size={12} />
            </button>
          </article>
        {/each}
      </div>
      <footer
        class="flex justify-start border-t border-[var(--color-border-subtle)] px-3 pb-2.5 pt-2"
      >
        <button
          type="button"
          class="border-0 bg-transparent px-1 py-[2px] text-[6px] font-extrabold tracking-[.02em] text-[var(--color-text-faint)] hover:text-[var(--color-text-secondary)]"
          onclick={onclear}
        >
          <Icon name="trash" size={13} />
          Clear all</button
        >
      </footer>
    </section>
  {/if}
</div>
