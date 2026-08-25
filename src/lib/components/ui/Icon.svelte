<script lang="ts">
  let {
    name,
    size = 20,
    strokeWidth = 1.8,
  }: { name: string; size?: number; strokeWidth?: number } = $props();
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width={strokeWidth}
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  {#if name === "home"}<path d="m3 11 9-8 9 8" /><path d="M5 10v10h14V10M9 20v-6h6v6" />
  {:else if name === "mods"}<path
      d="M8.5 3.5 12 7l3.5-3.5 5 5L17 12l3.5 3.5-5 5L12 17l-3.5 3.5-5-5L7 12 3.5 8.5z"
    /><circle cx="12" cy="12" r="2.2" />
  {:else if name === "modpacks"}<path d="m4 7 8-4 8 4-8 4z" /><path
      d="m4 7v10l8 4 8-4V7M12 11v10"
    /><path d="m8 9 8-4" />
  {:else if name === "instances"}<rect x="3" y="4" width="18" height="16" rx="3" /><path
      d="M8 4v16M8 9h13"
    />
  {:else if name === "settings"}<circle cx="12" cy="12" r="3" /><path
      d="M19.4 15a1.7 1.7 0 0 0 .3 1.9l.1.1-2.8 2.8-.1-.1a1.7 1.7 0 0 0-1.9-.3 1.7 1.7 0 0 0-1 1.6v.2h-4V21a1.7 1.7 0 0 0-1-1.6 1.7 1.7 0 0 0-1.9.3l-.1.1L4.2 17l.1-.1a1.7 1.7 0 0 0 .3-1.9A1.7 1.7 0 0 0 3 14H2.8v-4H3a1.7 1.7 0 0 0 1.6-1 1.7 1.7 0 0 0-.3-1.9L4.2 7 7 4.2l.1.1A1.7 1.7 0 0 0 9 4.6a1.7 1.7 0 0 0 1-1.6v-.2h4V3a1.7 1.7 0 0 0 1 1.6 1.7 1.7 0 0 0 1.9-.3l.1-.1L19.8 7l-.1.1a1.7 1.7 0 0 0-.3 1.9 1.7 1.7 0 0 0 1.6 1h.2v4H21a1.7 1.7 0 0 0-1.6 1Z"
    />
  {:else if name === "play"}<path d="m9 7 8 5-8 5Z" />
  {:else if name === "search"}<circle cx="11" cy="11" r="7" /><path d="m20 20-4-4" />
  {:else if name === "chevron"}<path d="m9 18 6-6-6-6" />
  {:else if name === "plus"}<path d="M12 5v14M5 12h14" />
  {:else if name === "download"}<path d="M12 3v12m0 0 4-4m-4 4-4-4M5 21h14" />
  {:else if name === "upload"}<path d="M12 16V4m0 0 4 4m-4-4-4 4M5 20h14" />
  {:else if name === "folder"}<path d="M3 6h7l2 2h9v11H3z" />
  {:else if name === "logs"}<path d="M4 4h16v16H4z" /><path d="m7 9 2 2-2 2M12 14h5" />
  {:else if name === "check"}<path d="m5 12 4 4L19 6" />
  {:else if name === "more"}<circle cx="5" cy="12" r="1" fill="currentColor" stroke="none" /><circle
      cx="12"
      cy="12"
      r="1"
      fill="currentColor"
      stroke="none"
    /><circle cx="19" cy="12" r="1" fill="currentColor" stroke="none" />
  {/if}
</svg>
