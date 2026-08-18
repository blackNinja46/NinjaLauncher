<script lang="ts">
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
    class: className = "",
    onchange,
    onopen,
  }: {
    value: SelectValue;
    options: SelectOption[];
    disabled?: boolean;
    ariaLabel: string;
    class?: string;
    onchange: (value: SelectValue) => void;
    onopen?: () => void;
  } = $props();

  let root: HTMLDivElement;
  let menu = $state<HTMLDivElement>();
  let open = $state(false);
  let activeIndex = $state(0);
  let menuPosition = $state("");
  const selectedIndex = $derived(options.findIndex((option) => option.value === value));
  const selectedLabel = $derived(options[selectedIndex]?.label ?? options[0]?.label ?? "Select…");

  function updatePosition() {
    if (!root) return;
    const rect = root.getBoundingClientRect();
    const menuHeight = Math.min(240, Math.max(40, options.length * 36 + 8));
    const openAbove = window.innerHeight - rect.bottom < menuHeight && rect.top > menuHeight;
    menuPosition = [
      `left:${Math.max(8, Math.min(rect.left, window.innerWidth - rect.width - 8))}px`,
      `width:${rect.width}px`,
      openAbove
        ? `bottom:${Math.max(8, window.innerHeight - rect.top + 4)}px`
        : `top:${Math.min(window.innerHeight - 8, rect.bottom + 4)}px`,
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
    root.querySelector("button")?.focus();
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
    class="mc-field flex size-full min-h-9 items-center gap-2 px-3 py-2 text-left text-inherit outline-none transition hover:border-cyan-300/40 disabled:cursor-not-allowed disabled:opacity-50"
  >
    <span class="min-w-0 flex-1 truncate">{selectedLabel}</span>
    <svg
      viewBox="0 0 16 16"
      aria-hidden="true"
      class={`size-3.5 shrink-0 text-cyan-300/70 transition-transform ${open ? "rotate-180" : ""}`}
    >
      <path d="m3 6 5 5 5-5" fill="none" stroke="currentColor" stroke-width="1.8" />
    </svg>
  </button>
</div>

{#if open}
  <div
    bind:this={menu}
    role="listbox"
    aria-label={ariaLabel}
    class="fixed z-[100] max-h-60 overflow-y-auto border border-cyan-300/30 bg-[#06152b]/[.98] p-1 shadow-[0_14px_38px_rgba(0,4,18,.72),inset_0_1px_0_rgba(105,217,255,.08)] backdrop-blur-xl"
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
        class="flex w-full items-center gap-2 px-2.5 py-2 text-left text-inherit transition disabled:opacity-40 {index ===
        activeIndex
          ? 'bg-cyan-300/[.10] text-cyan-50'
          : 'text-[#9bc3df] hover:bg-white/[.05]'}"
      >
        <span
          class="size-1.5 shrink-0 {option.value === value
            ? 'bg-cyan-300 shadow-[0_0_7px_#19d8f1]'
            : 'bg-transparent'}"
        ></span>
        <span class="min-w-0 flex-1 truncate">{option.label}</span>
      </button>
    {/each}
  </div>
{/if}
