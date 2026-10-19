<script lang="ts">
  import Icon from "$lib/components/ui/Icon.svelte";

  export type SelectValue = string | number;
  export type SelectOption = {
    value: SelectValue;
    label: string;
    disabled?: boolean;
  };

  let {
    value,
    options,
    disabled = false,
    ariaLabel,
    displayLabel,
    class: className = "",
    onchange,
    onopen,
  }: {
    value: SelectValue;
    options: SelectOption[];
    disabled?: boolean;
    ariaLabel: string;
    displayLabel?: string;
    class?: string;
    onchange: (value: SelectValue) => void;
    onopen?: () => void;
  } = $props();

  let root: HTMLDivElement;
  let menu = $state<HTMLDivElement>();
  let open = $state(false);
  let activeIndex = $state(0);
  let menuPosition = $state("");
  let opensAbove = $state(false);
  const selectedIndex = $derived(options.findIndex((option) => option.value === value));
  const selectedLabel = $derived(options[selectedIndex]?.label ?? options[0]?.label ?? "Select…");
  const triggerLabel = $derived(displayLabel ?? selectedLabel);

  function updatePosition() {
    if (!root) return;
    const rect = root.getBoundingClientRect();
    const menuHeight = Math.min(320, Math.max(56, options.length * 52 + 16));
    const menuWidth = Math.min(Math.max(rect.width, 380), window.innerWidth - 24);
    opensAbove = window.innerHeight - rect.bottom < menuHeight && rect.top > menuHeight;
    menuPosition = [
      `left:${Math.max(12, Math.min(rect.left, window.innerWidth - menuWidth - 12))}px`,
      `width:${menuWidth}px`,
      "right:auto",
      opensAbove
        ? `top:auto;bottom:${Math.max(12, window.innerHeight - rect.top + 8)}px`
        : `top:${Math.min(window.innerHeight - 12, rect.bottom + 8)}px;bottom:auto`,
    ].join(";");
  }

  function show() {
    if (disabled) return;
    activeIndex = Math.max(0, selectedIndex);
    open = true;
    updatePosition();
    onopen?.();
  }

  function choose(option: SelectOption) {
    if (option.disabled) return;
    onchange(option.value);
    open = false;
    root.querySelector("button")?.focus({ preventScroll: true });
  }

  function move(direction: 1 | -1) {
    if (!options.length) return;
    let next = activeIndex;
    do next = (next + direction + options.length) % options.length;
    while (options[next]?.disabled && next !== activeIndex);
    activeIndex = next;
  }

  function handleKeydown(event: KeyboardEvent) {
    if (disabled) return;
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      if (!open) show();
      else move(event.key === "ArrowDown" ? 1 : -1);
    } else if (event.key === "Home" && open) {
      event.preventDefault();
      activeIndex = 0;
    } else if (event.key === "End" && open) {
      event.preventDefault();
      activeIndex = Math.max(0, options.length - 1);
    } else if ((event.key === "Enter" || event.key === " ") && open) {
      event.preventDefault();
      const option = options[activeIndex];
      if (option) choose(option);
    } else if (event.key === "Escape" && open) {
      event.preventDefault();
      open = false;
    } else if (event.key === "Tab") {
      open = false;
    }
  }

  $effect(() => {
    if (!open) return;
    const currentMenu = menu;
    if (currentMenu && !currentMenu.matches(":popover-open")) {
      currentMenu.showPopover();
      requestAnimationFrame(() => {
        currentMenu
          .querySelector<HTMLElement>('[aria-selected="true"]')
          ?.scrollIntoView({ block: "nearest" });
      });
    }
    const closeOutside = (event: PointerEvent) => {
      const target = event.target as Node;
      if (!root.contains(target) && !menu?.contains(target)) open = false;
    };
    const reposition = () => updatePosition();
    document.addEventListener("pointerdown", closeOutside);
    window.addEventListener("resize", reposition);
    window.addEventListener("scroll", reposition, true);
    return () => {
      document.removeEventListener("pointerdown", closeOutside);
      window.removeEventListener("resize", reposition);
      window.removeEventListener("scroll", reposition, true);
    };
  });
</script>

<div bind:this={root} class={`relative ${className}`}>
  <button
    type="button"
    {disabled}
    aria-label={ariaLabel}
    aria-haspopup="listbox"
    aria-expanded={open}
    onkeydown={handleKeydown}
    onclick={() => (open ? (open = false) : show())}
    class="launcher-select-trigger group/select relative flex size-full min-h-10 items-center gap-2 overflow-hidden rounded-[6px] border border-white/[.13] bg-ink-900/72 px-3 py-2 text-left text-inherit shadow-[inset_0_1px_0_rgba(255,255,255,.1),0_8px_22px_rgba(0,4,18,.16)] outline-none backdrop-blur-xl transition hover:border-accent-400/40 hover:bg-ink-850/82 disabled:cursor-not-allowed disabled:opacity-50"
  >
    <span class="min-w-0 flex-1 truncate text-sm">{triggerLabel}</span>
    <span
      class={`shrink-0 text-cyan-200/75 transition-transform duration-300 ${open ? "rotate-180" : ""}`}
    >
      <Icon name="chevron-down" size={12} />
    </span>
  </button>
</div>

{#if open}
  <div
    bind:this={menu}
    popover="manual"
    role="listbox"
    aria-label={ariaLabel}
    class="select-menu fixed z-[100] m-0 max-h-80 overflow-y-auto rounded-[9px] p-2 text-[var(--color-text-primary)] shadow-[0_22px_60px_rgba(0,4,18,.82),inset_0_1px_0_rgba(153,231,255,.14)] outline outline-1 outline-black/30 backdrop-blur-3xl {opensAbove
      ? 'origin-bottom'
      : 'origin-top'}"
    style={menuPosition}
  >
    {#each options as option, index (option.value)}
      <button
        type="button"
        role="option"
        aria-selected={option.value === value}
        disabled={option.disabled}
        onmouseenter={() => (activeIndex = index)}
        onclick={() => choose(option)}
        class="group/option flex w-full items-center gap-3 rounded-[6px] border px-3 py-2.5 text-left text-xs font-semibold text-inherit disabled:opacity-40 {index ===
        activeIndex
          ? 'border-cyan-600/20 bg-gradient-to-r from-cyan-300/[.16] to-blue-500/[.08] text-white shadow-[inset_0_1px_0_rgba(255,255,255,.06)]'
          : 'border-transparent text-[var(--color-text-secondary)] hover:border-white/[.07] hover:bg-white/[.055] hover:text-white'}"
      >
        <span
          class="grid size-4 shrink-0 place-items-center border {option.value === value
            ? 'border-cyan-600/45 bg-cyan-300/15 text-cyan-200'
            : 'border-white/[.08] bg-black/10 text-transparent'}"
        >
          <Icon name="check" size={9} />
        </span>
        <span class="min-w-0 flex-1 whitespace-normal break-words leading-5">{option.label}</span>
      </button>
    {/each}
  </div>
{/if}

<style>
  .select-menu:popover-open {
    animation: select-open 180ms cubic-bezier(0.16, 1, 0.3, 1);
    overscroll-behavior: contain;
  }

  @keyframes select-open {
    from {
      opacity: 0;
      transform: translateY(-5px) scale(0.975);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }

  .select-menu {
    background: linear-gradient(
      145deg,
      color-mix(in srgb, var(--color-navy-700) 96%, transparent),
      color-mix(in srgb, var(--color-navy-950) 97%, transparent) 52%
    );
    scrollbar-width: thin;
    scrollbar-color:
      var(--color-cyan-200) 80%,
      transparent;
  }
</style>
