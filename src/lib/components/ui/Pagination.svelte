<script lang="ts">
  let {
    page,
    total,
    loading = false,
    onchange,
  }: {
    page: number;
    total: number;
    loading?: boolean;
    onchange: (page: number) => void;
  } = $props();

  const pageCount = $derived(Math.max(1, Math.ceil(total / 20)));
  const visiblePages = $derived.by(() => {
    const start = Math.max(0, Math.min(page - 2, pageCount - 5));
    return Array.from({ length: Math.min(5, pageCount) }, (_, index) => start + index);
  });
</script>

{#if pageCount > 1}
  <nav class="mt-5 flex items-center justify-center gap-1.5" aria-label="Search result pages">
    <button
      type="button"
      onclick={() => onchange(page - 1)}
      disabled={loading || page === 0}
      class="mc-secondary px-3 py-2 text-[10px] font-bold uppercase tracking-wider disabled:cursor-not-allowed disabled:opacity-35"
      aria-label="Previous page"
    >
      Previous
    </button>
    {#each visiblePages as pageIndex}
      <button
        type="button"
        onclick={() => onchange(pageIndex)}
        disabled={loading}
        aria-label={`Page ${pageIndex + 1}`}
        aria-current={pageIndex === page ? "page" : undefined}
        class="grid size-9 place-items-center border text-[11px] font-bold transition disabled:cursor-wait {pageIndex ===
        page
          ? 'border-cyan-300/65 bg-cyan-300/15 text-cyan-50 shadow-[inset_0_-2px_0_#19d8f1,0_0_12px_rgba(25,216,241,.12)]'
          : 'border-cyan-300/15 bg-[#07172c] text-[#7295b2] hover:border-cyan-300/35 hover:text-cyan-100'}"
      >
        {pageIndex + 1}
      </button>
    {/each}
    <button
      type="button"
      onclick={() => onchange(page + 1)}
      disabled={loading || page >= pageCount - 1}
      class="mc-secondary px-3 py-2 text-[10px] font-bold uppercase tracking-wider disabled:cursor-not-allowed disabled:opacity-35"
      aria-label="Next page"
    >
      Next
    </button>
    <span class="ml-2 text-[10px] text-[#55748f]">{total.toLocaleString()} results</span>
  </nav>
{/if}
