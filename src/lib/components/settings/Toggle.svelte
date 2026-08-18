<script lang="ts">
  let { checked = $bindable(false), label }: { checked?: boolean; label: string } = $props();
</script>

<button
  type="button"
  role="switch"
  aria-checked={checked}
  aria-label={label}
  onclick={() => (checked = !checked)}
  class="group relative inline-flex h-7 w-[58px] shrink-0 items-center overflow-hidden rounded-md border transition-all duration-200 {checked
    ? 'border-cyan-200/65 bg-gradient-to-r from-[#0874d4] to-[#12b9df] shadow-[inset_0_1px_0_rgba(190,248,255,.32),0_0_14px_rgba(25,216,241,.18)]'
    : 'border-cyan-300/20 bg-[#07182d] shadow-[inset_0_2px_3px_rgba(0,0,0,.35)] hover:border-cyan-300/35'}"
>
  <span
    aria-hidden="true"
    class="absolute text-[8px] font-extrabold uppercase tracking-[.1em] transition-colors {checked
      ? 'left-2 text-cyan-50'
      : 'right-1.5 text-[#6688a6]'}"
  >
    {checked ? "ON" : "OFF"}
  </span>
  <span
    aria-hidden="true"
    class="absolute left-1 size-[18px] rounded-sm border transition-all duration-200 group-active:scale-90 {checked
      ? 'translate-x-[32px] border-white/80 bg-white shadow-[0_0_9px_rgba(201,250,255,.72)]'
      : 'translate-x-0 border-[#6685a0]/50 bg-[#b8c6d2] shadow-[0_2px_5px_rgba(0,0,0,.4)]'}"
  >
  </span>
</button>
