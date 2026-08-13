<script lang="ts">
  let { checked = $bindable(false), label }: { checked?: boolean; label: string } = $props();
</script>

<button
  role="switch"
  aria-checked={checked}
  aria-label={label}
  onclick={() => (checked = !checked)}
  class:bg-accent-500={checked}
  class="relative h-6 w-11 border border-cyan-300/20 bg-[#12304f] transition"
>
  <span
    class:translate-x-5={checked}
    class="absolute left-1 top-1 size-4 bg-white shadow-[0_0_8px_rgba(25,216,241,.5)] transition"
  >
  </span>
</button>
