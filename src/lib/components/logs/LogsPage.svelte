<script lang="ts">
  import { tick } from "svelte";
  import PageHeader from "$lib/components/layout/PageHeader.svelte";
  import { openLogsFolder } from "$lib/api/tauri";
  import type { GameLogEntry } from "$lib/types";

  type LogLevel = "info" | "warning" | "error" | "debug";
  type LogFilter = "all" | LogLevel;

  let {
    logs,
    running,
    onclear,
  }: {
    logs: GameLogEntry[];
    running: boolean;
    onclear: () => void;
  } = $props();
  let consoleElement: HTMLDivElement;
  let follow = $state(true);
  let filter = $state<LogFilter>("all");

  const filters: { id: LogFilter; label: string }[] = [
    { id: "all", label: "All" },
    { id: "info", label: "Info" },
    { id: "warning", label: "Warnings" },
    { id: "error", label: "Errors" },
    { id: "debug", label: "Debug" },
  ];

  function levelFor(entry: GameLogEntry): LogLevel {
    const line = entry.line.toLowerCase();
    if (
      /\[(error|fatal)\]|\b(error|fatal)\b|exception|caused by:|crash report|^\s*at\s+|^\s*\.\.\. \d+ more/.test(
        line,
      )
    )
      return "error";
    if (/\[(warn|warning)\]|\bwarn(?:ing)?\b/.test(line)) return "warning";
    if (/\[(debug|trace)\]|\bdebug\b|\btrace\b/.test(line)) return "debug";
    return "info";
  }

  function levelClasses(level: LogLevel): string {
    if (level === "error") return "border-red-400/55";
    if (level === "warning") return "border-amber-400/45";
    if (level === "debug") return "border-cyan-500/40";
    return "border-white/10";
  }

  let categorized = $derived(logs.map((entry) => ({ entry, level: levelFor(entry) })));
  let visibleLogs = $derived(
    filter === "all" ? categorized : categorized.filter((item) => item.level === filter),
  );
  let counts = $derived({
    all: categorized.length,
    info: categorized.filter((item) => item.level === "info").length,
    warning: categorized.filter((item) => item.level === "warning").length,
    error: categorized.filter((item) => item.level === "error").length,
    debug: categorized.filter((item) => item.level === "debug").length,
  });

  $effect(() => {
    visibleLogs.length;
    if (follow) tick().then(() => consoleElement?.scrollTo({ top: consoleElement.scrollHeight }));
  });
</script>

<div class="flex items-start justify-between">
  <PageHeader
    title="Game Logs"
    description="Live Minecraft output for troubleshooting mods, Java, and crashes."
  />
  <div class="flex gap-3">
    <button onclick={openLogsFolder} class="mc-secondary px-4 py-2.5 text-xs font-bold text-white"
      >Open log folder</button
    >
    <button onclick={onclear} class="mc-secondary px-4 py-2.5 text-xs font-bold text-white"
      >Clear</button
    >
  </div>
</div>

<section class="mc-panel overflow-hidden">
  <div class="flex items-center gap-3 border-b border-white/[0.08] bg-white/[0.02] px-4 py-3">
    <span class="size-2 {running ? 'bg-emerald-400 shadow-[0_0_8px_#34d399]' : 'bg-slate-500'}"
    ></span>
    <span class="text-[10px] font-bold uppercase tracking-[.14em] text-[var(--color-text-secondary)]"
      >{running ? "Minecraft running" : "Minecraft stopped"}</span
    >
    <label
      class="ml-auto flex items-center gap-2 text-[10px] uppercase tracking-wider text-[var(--color-text-muted)]"
      ><input type="checkbox" bind:checked={follow} class="accent-[var(--color-primary)]" /> Follow output</label
    >
    <span class="text-[10px] tabular-nums text-[var(--color-text-faint)]"
      >{visibleLogs.length} / {logs.length} lines</span
    >
  </div>
  <div
    class="flex flex-wrap items-center gap-2 border-b border-white/[0.08] bg-white/[0.01] px-4 py-3"
    aria-label="Log categories"
  >
    {#each filters as item}
      <button
        onclick={() => (filter = item.id)}
        aria-pressed={filter === item.id}
        class="flex items-center gap-2 border px-3 py-1.5 text-[10px] font-bold uppercase tracking-[.08em] transition {filter ===
        item.id
          ? 'border-white/[0.16] bg-white/[0.09] text-white shadow-[inset_2px_0_0_var(--color-primary)]'
          : 'border-white/[0.08] bg-white/[0.025] text-[var(--color-text-muted)] hover:border-white/[0.16] hover:bg-white/[0.05] hover:text-white'}"
      >
        <span
          class="size-1.5 {item.id === 'error'
            ? 'bg-red-400'
            : item.id === 'warning'
              ? 'bg-amber-400'
              : item.id === 'debug'
              ? 'bg-cyan-500'
              : 'bg-slate-300'}"
        ></span>
        {item.label}<span class="text-[9px] opacity-60">{counts[item.id]}</span>
      </button>
    {/each}
  </div>
  <div
    bind:this={consoleElement}
    class="h-[450px] overflow-auto bg-black/20 p-4 font-mono text-[11px] leading-[1.65] text-[var(--color-text-secondary)] selection:bg-[var(--color-brand-selection)]"
  >
    {#if visibleLogs.length}
      {#each visibleLogs as item}
        <div
          class="group flex border-l-2 px-3 py-px transition-colors hover:bg-white/[0.025] {levelClasses(
            item.level,
          )}"
        >
          <span class="mr-3 w-12 shrink-0 select-none text-[9px] font-bold uppercase opacity-45"
            >{item.level === "warning" ? "WARN" : item.level}</span
          >
          <span class="whitespace-pre-wrap break-all {item.level === 'error' ? "text-red-400" : item.level === "warning" ? "text-amber-400" : item.level === "debug" ? "text-cyan-400" : "text-slate-400"}">{item.entry.line}</span>
        </div>
      {/each}
    {:else if logs.length}
      <div class="grid h-full place-items-center text-center text-[var(--color-text-faint)]">
        <div>
          <div class="text-sm font-bold uppercase tracking-wider text-[var(--color-text-muted)]">
            No {filter} messages
          </div>
          <p class="mt-2 text-xs">Choose another category to see more output.</p>
        </div>
      </div>
    {:else}
      <div class="grid h-full place-items-center text-center text-[var(--color-text-faint)]">
        <div>
          <div class="text-sm font-bold uppercase tracking-wider text-[var(--color-text-muted)]">
            Waiting for Minecraft :/
          </div>
          <p class="mt-2 text-xs">Game output will appear here after launch.</p>
        </div>
      </div>
    {/if}
  </div>
</section>
