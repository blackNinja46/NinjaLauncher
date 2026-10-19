<script lang="ts">
  import { fly } from "svelte/transition";
  import Icon from "$lib/components/ui/Icon.svelte";
  import InlineMessage from "$lib/components/ui/InlineMessage.svelte";

  type ToastTone = "info" | "success" | "warning" | "error";
  type ToastItem = {
    id: string;
    title: string;
    message: string;
    tone: ToastTone;
  };

  let { items, ondismiss }: { items: ToastItem[]; ondismiss: (id: string) => void } = $props();
  let layer = $state<HTMLDivElement>();
  let visibleKey = $state("");

  const styles: Record<ToastTone, { icon: string; accent: string }> = {
    info: {
      icon: "info",
      accent: "text-[var(--color-primary)] bg-[var(--color-primary)]/15",
    },
    success: {
      icon: "check",
      accent: "text-[var(--color-success)] bg-[var(--color-success)]/15",
    },
    warning: {
      icon: "info",
      accent: "text-[var(--color-warning)] bg-[var(--color-warning)]/15",
    },
    error: {
      icon: "info",
      accent: "text-[var(--color-danger)] bg-[var(--color-danger)]/15",
    },
  };

  $effect(() => {
    const nextKey = items.map((item) => item.id).join(":");
    if (!layer || nextKey === visibleKey) return;
    visibleKey = nextKey;
    queueMicrotask(() => {
      if (!layer) return;
      if (layer.matches(":popover-open")) layer.hidePopover();
      layer.showPopover();
    });
  });
</script>

<div
  bind:this={layer}
  popover="manual"
  class="pointer-events-none fixed bottom-auto left-auto right-4 top-14 m-0 flex w-[min(360px,calc(100vw-32px))] flex-col gap-2.5 overflow-visible border-0 bg-transparent p-0"
  aria-live="polite"
  aria-label="Status messages"
>
  {#each items as item (item.id)}
    {@const style = styles[item.tone]}
    <article
      class="pointer-events-auto flex items-start gap-3 rounded-[10px] bg-[#0d0f14]/10 p-3.5 text-[var(--color-text-primary)] shadow-[0_24px_70px_rgba(0,0,0,.46),inset_0_1px_0_rgba(255,255,255,.07)] backdrop-blur-md"
      in:fly={{ x: 28, duration: 220 }}
      out:fly={{ x: 28, duration: 160 }}
    >
      <span class="grid size-8 shrink-0 place-items-center rounded-[6px] {style.accent}">
        <Icon name={style.icon} size={14} />
      </span>
      <div class="min-w-0 flex-1">
        <h2 class="text-xs font-bold">{item.title}</h2>
        <p class="mt-1 text-[11px] leading-[1.55] text-[var(--color-text-muted)]">
          <InlineMessage message={item.message} />
        </p>
      </div>
      <button
        type="button"
        class="grid size-7 shrink-0 place-items-center rounded-[5px] border-0 bg-transparent text-[var(--color-text-faint)] hover:bg-white/[.07] hover:text-white"
        aria-label={`Close ${item.title}`}
        onclick={() => ondismiss(item.id)}
      >
        <Icon name="close" size={12} />
      </button>
    </article>
  {/each}
</div>
