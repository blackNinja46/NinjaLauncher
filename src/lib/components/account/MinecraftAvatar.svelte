<script lang="ts">
  import type { Account } from "$lib/types";

  let { account }: { account: Account | null } = $props();
  let failed = $state(false);

  const avatarUrl = $derived(
    account ? `https://api.mcheads.org/head/${encodeURIComponent(account.uuid)}/64` : "",
  );
</script>

<div
  class="relative grid size-full place-items-center overflow-hidden bg-[#0a3b68] font-bold text-cyan-100 rounded-lg"
>
  <span aria-hidden="true">{account ? (account.username[0]?.toUpperCase() ?? "?") : "?"}</span>
  {#if account && !failed}
    <img
      src={avatarUrl}
      alt={`${account.username}'s Minecraft skin head`}
      class="absolute inset-0 size-full object-cover [image-rendering:pixelated]"
      onerror={() => (failed = true)}
    />
  {/if}
</div>
