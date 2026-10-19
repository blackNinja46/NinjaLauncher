<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";

  let {
    message,
    linkClass = "text-[var(--color-primary)]",
  }: {
    message: string;
    linkClass?: string;
  } = $props();

  type MessagePart = {
    type: "text" | "link";
    value: string;
    href?: string;
  };

  let parts = $derived(parseMessage(message));

  function parseMessage(value: string): MessagePart[] {
    const parts: MessagePart[] = [];
    const token =
      /<\/?(?:red|blue|green|yellow|orange|purple)>|\[([^\]]+)]\\?\((https?:\/\/[^\s)]+)\\?\)/gi;
    let cursor = 0;
    let match: RegExpExecArray | null;
    while ((match = token.exec(value))) {
      if (match.index > cursor) {
        parts.push({ type: "text", value: value.slice(cursor, match.index) });
      }
      if (!match[0].startsWith("<")) {
        parts.push({ type: "link", value: match[1], href: match[2] });
      }
      cursor = token.lastIndex;
    }
    if (cursor < value.length) parts.push({ type: "text", value: value.slice(cursor) });
    return parts;
  }

  async function openExternalLink(url: string) {
    if (!/^https?:\/\//i.test(url)) return;
    if ("__TAURI_INTERNALS__" in window) await openUrl(url);
    else window.open(url, "_blank", "noopener,noreferrer");
  }
</script>

{#each parts as part}
  {#if part.type === "link" && part.href}
    <button
      type="button"
      onclick={() => openExternalLink(part.href!)}
      class="mx-0.5 inline border-0 bg-transparent p-0 font-bold {linkClass} underline decoration-current/45 underline-offset-[3px] hover:brightness-125"
      >{part.value}</button
    >
  {:else}
    <span>{part.value}</span>
  {/if}
{/each}
