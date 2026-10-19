<script lang="ts">
  import Icon from "$lib/components/ui/Icon.svelte";
  import { isNameGradientUnlocked, nameGradients, type NameGradient } from "$lib/nameGradients";
  import type { PlayerRanks } from "$lib/types";

  let {
    username,
    ranks,
    selected,
    onselect,
    onclose,
  }: {
    username: string;
    ranks: PlayerRanks | null;
    selected: string;
    onselect: (gradient: NameGradient) => void;
    onclose: () => void;
  } = $props();
</script>

<svelte:window onkeydown={(event) => event.key === "Escape" && onclose()} />

<div
  class="fixed inset-0 z-[105] grid place-items-center bg-black/55 p-6 backdrop-blur-lg"
  role="presentation"
  onclick={(event) => event.currentTarget === event.target && onclose()}
>
  <div
    class="gradient-dialog w-full max-w-[540px] overflow-hidden rounded-[13px] border border-white/[.11] bg-[var(--color-section-background)] shadow-[0_28px_90px_rgba(0,0,0,.68)]"
    role="dialog"
    aria-modal="true"
    aria-labelledby="gradient-dialog-title"
  >
    <header class="flex items-start justify-between border-b border-white/[.08] px-6 py-5">
      <div>
        <p class="text-[9px] font-bold uppercase tracking-[.16em] text-[var(--color-primary)]">
          Player appearance
        </p>
        <h2 id="gradient-dialog-title" class="mt-1 text-lg font-black text-white">
          Animated name gradient
        </h2>
        <p class="mt-1 text-xs text-[var(--color-text-muted)]">
          Choose the style assigned to one of your ranks. Admin unlocks every style.
        </p>
      </div>
      <button
        type="button"
        onclick={onclose}
        class="mc-secondary grid size-9 shrink-0 place-items-center"
        aria-label="Close gradient selection"
      >
        <Icon name="close" size={14} />
      </button>
    </header>

    <div class="grid max-h-[430px] grid-cols-2 gap-2.5 overflow-y-auto p-5">
      {#each nameGradients as gradient (gradient.id)}
        {@const unlocked = isNameGradientUnlocked(gradient, ranks)}
        <button
          type="button"
          disabled={!unlocked}
          onclick={() => unlocked && onselect(gradient)}
          class="gradient-option group relative min-h-[82px] overflow-hidden rounded-[9px] border p-3 text-left transition {selected ===
          gradient.id
            ? 'border-[var(--color-primary)] bg-[var(--color-primary)]/[.09] shadow-[0_0_0_1px_color-mix(in_srgb,var(--color-primary)_24%,transparent)]'
            : unlocked
              ? 'border-white/[.08] bg-white/[.025] hover:-translate-y-0.5 hover:border-white/[.16] hover:bg-white/[.055]'
              : 'cursor-not-allowed border-white/[.05] bg-black/10 opacity-45'}"
        >
          <div class="flex items-center justify-between gap-3">
            <span class="text-[9px] font-bold uppercase tracking-[.13em] text-white/55">
              {gradient.label}
            </span>
            {#if selected === gradient.id}
              <span class="text-[var(--color-primary)]"><Icon name="check" size={12} /></span>
            {:else if !unlocked}
              <span class="text-white/40"><Icon name="lock" size={11} /></span>
            {/if}
          </div>
          <div
            class:animated-gradient-preview={gradient.id !== "none" && unlocked}
            class="mt-2 truncate text-lg font-black tracking-[-.03em] {gradient.id === 'none'
              ? 'text-white'
              : ''}"
            style={gradient.id === "none"
              ? undefined
              : `--gradient-colors: ${gradient.colors.join(", ")};`}
          >
            {username}
          </div>
          {#if !unlocked}
            <span class="mt-1 block text-[9px] text-white/35">Requires {gradient.label}</span>
          {/if}
        </button>
      {/each}
    </div>
  </div>
</div>

<style>
  .gradient-dialog {
    animation: gradient-dialog-in 260ms cubic-bezier(0.16, 1, 0.3, 1) both;
  }

  .animated-gradient-preview {
    color: transparent;
    background-image: linear-gradient(90deg, var(--gradient-colors));
    background-size: 160% 100%;
    background-clip: text;
    -webkit-background-clip: text;
    filter: saturate(1.25) brightness(1.1)
      drop-shadow(0 2px 6px color-mix(in srgb, var(--gradient-glow) 32%, transparent));
    animation: name-gradient-flow 1.6s ease-in-out infinite alternate;
  }

  @keyframes gradient-dialog-in {
    from {
      opacity: 0;
      transform: translateY(14px) scale(0.975);
    }
  }

  @keyframes name-gradient-flow {
    from {
      background-position: 0% 50%;
    }
    to {
      background-position: 100% 50%;
    }
  }

  :global(.animations-disabled) .gradient-dialog,
  :global(.animations-disabled) .animated-gradient-preview {
    animation: none;
  }

  @media (prefers-reduced-motion: reduce) {
    .gradient-dialog,
    .animated-gradient-preview {
      animation: none;
    }
  }
</style>
