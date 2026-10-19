<script lang="ts">
  import Icon from "$lib/components/ui/Icon.svelte";
  import InlineMessage from "$lib/components/ui/InlineMessage.svelte";
  import type { ClientAnnouncement } from "$lib/types";

  let {
    announcement,
    ondismiss,
  }: {
    announcement: ClientAnnouncement;
    ondismiss: (id: string) => void;
  } = $props();

  type BannerColor = "red" | "blue" | "green" | "yellow" | "orange" | "purple";

  const colorClasses: Record<
    BannerColor,
    { container: string; icon: string; detail: string; close: string }
  > = {
    red: {
      container: "border-red-400/35 bg-red-950/95",
      icon: "bg-red-300/15 text-red-200",
      detail: "text-red-100/80",
      close: "text-red-100/55 hover:bg-white/10 hover:text-white",
    },
    blue: {
      container: "border-blue-400/35 bg-blue-950/95",
      icon: "bg-blue-300/15 text-blue-200",
      detail: "text-blue-100/80",
      close: "text-blue-100/55 hover:bg-white/10 hover:text-white",
    },
    green: {
      container: "border-green-400/35 bg-green-950/95",
      icon: "bg-green-300/15 text-green-200",
      detail: "text-green-100/80",
      close: "text-green-100/55 hover:bg-white/10 hover:text-white",
    },
    yellow: {
      container: "border-amber-400/40 bg-amber-950/95",
      icon: "bg-amber-300/15 text-amber-200",
      detail: "text-amber-100/80",
      close: "text-amber-100/55 hover:bg-white/10 hover:text-white",
    },
    orange: {
      container: "border-orange-400/40 bg-orange-950/95",
      icon: "bg-orange-300/15 text-orange-200",
      detail: "text-orange-100/80",
      close: "text-orange-100/55 hover:bg-white/10 hover:text-white",
    },
    purple: {
      container: "border-purple-400/35 bg-purple-950/95",
      icon: "bg-purple-300/15 text-purple-200",
      detail: "text-purple-100/80",
      close: "text-purple-100/55 hover:bg-white/10 hover:text-white",
    },
  };

  let isWarning = $derived(announcement.priority === "CRITICAL" || announcement.type === "WARNING");
  let bannerColor = $derived(colorFor(announcement.message));
  let colors = $derived(colorClasses[bannerColor]);

  function colorFor(message: string): BannerColor {
    const requested = message.match(/<(red|blue|green|yellow|orange|purple)>/i)?.[1];
    if (requested) return requested.toLowerCase() as BannerColor;
    return isWarning ? "red" : "blue";
  }
</script>

<section
  class="announcement-banner grid min-h-7 w-full shrink-0 grid-cols-[20px_minmax(0,1fr)_20px] items-center gap-2 overflow-hidden px-4 py-0.5 shadow-[0_6px_18px_rgba(0,0,0,.24)] backdrop-blur-xl {colors.container}"
  role={isWarning ? "alert" : "status"}
  aria-label={announcement.title}
>
  <span class="grid size-5 shrink-0 place-items-center rounded-[4px] {colors.icon}">
    <Icon name={isWarning ? "warning" : "info"} size={9} />
  </span>
  <div class="min-w-0 text-center text-[12px] leading-3.5">
    <strong class="mr-2 text-white">{announcement.title}</strong>
    <span class={colors.detail}>
      <InlineMessage message={announcement.message} linkClass="text-white" />
    </span>
  </div>
  <button
    type="button"
    onclick={() => ondismiss(announcement.id)}
    class="grid size-5 shrink-0 place-items-center border-0 bg-transparent {colors.close}"
    aria-label={`Dismiss ${announcement.title}`}
    title="Dismiss"><Icon name="close" size={9} /></button
  >
</section>

<style>
  .announcement-banner {
    animation: banner-enter 280ms cubic-bezier(0.16, 1, 0.3, 1) both;
  }
  @keyframes banner-enter {
    from {
      opacity: 0;
      transform: translateY(-10px) scale(0.985);
    }
  }
  :global(.animations-disabled) .announcement-banner {
    animation: none;
  }
  @media (prefers-reduced-motion: reduce) {
    .announcement-banner {
      animation: none;
    }
  }
</style>
