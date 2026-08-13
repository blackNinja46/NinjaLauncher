<script lang="ts">
  import type { Snippet } from "svelte";
  let { title, description, children }: { title: string; description: string; children: Snippet } =
    $props();
</script>

<div
  class="flex min-h-[74px] items-center gap-6 border-b border-cyan-300/[0.09] px-5 py-4 last:border-0 hover:bg-cyan-300/[0.018]"
>
  <div class="min-w-0 flex-1">
    <div class="text-[13px] font-semibold text-[#e2f3ff]">{title}</div>
    <div class="mt-1 text-[11px] leading-relaxed text-[#6685a4]">{description}</div>
  </div>
  <div class="shrink-0">{@render children()}</div>
</div>
