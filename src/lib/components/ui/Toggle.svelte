<script lang="ts">
  let {
    checked = $bindable(false),
    label,
    disabled = false,
    showState = false,
    onLabel = "On",
    offLabel = "Off",
    onchange,
  }: {
    checked?: boolean;
    label: string;
    disabled?: boolean;
    showState?: boolean;
    onLabel?: string;
    offLabel?: string;
    onchange?: (checked: boolean) => void | Promise<void>;
  } = $props();

  function toggle() {
    if (disabled) return;
    const next = !checked;
    if (onchange) void onchange(next);
    else checked = next;
  }
</script>

<button
  type="button"
  role="switch"
  aria-checked={checked}
  aria-label={label}
  {disabled}
  onclick={toggle}
  class="group/toggle inline-flex shrink-0 items-center gap-2.5 text-left disabled:cursor-wait disabled:opacity-45"
>
  <span
    aria-hidden="true"
    class="relative h-6 w-11 shrink-0 rounded-full border transition-colors duration-200 {checked
      ? 'border-cyan-300/70 bg-cyan-300'
      : 'border-white/[.16] bg-white/[.055] group-hover/toggle:border-white/[.28]'}"
  >
    <span
      class="absolute left-[3px] top-[3px] size-4 rounded-full transition-all duration-200 ease-out group-active/toggle:scale-90 {checked
        ? 'translate-x-5 bg-white shadow-[0_1px_4px_rgba(0,0,0,.28)]'
        : 'translate-x-0 bg-[var(--color-text-muted)]'}"
    ></span>
  </span>
  {#if showState}
    <span
      class="min-w-16 text-[9px] font-bold uppercase tracking-[.1em] transition-colors {checked
        ? 'text-[var(--color-text-primary)]'
        : 'text-[var(--color-text-faint)]'}"
    >
      {checked ? onLabel : offLabel}
    </span>
  {/if}
</button>
