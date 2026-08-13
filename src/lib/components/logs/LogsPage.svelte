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
    if (level === "error") return "border-red-400/70 bg-red-400/[0.045] text-red-200";
    if (level === "warning") return "border-amber-400/65 bg-amber-400/[0.035] text-amber-100/90";
    if (level === "debug") return "border-violet-400/55 bg-violet-400/[0.03] text-violet-200/80";
    return "border-cyan-400/25 text-[#9dc4df]";
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
    <button onclick={openLogsFolder} class="mc-secondary px-4 py-2.5 text-xs font-bold text-cyan-50"
      >Open log folder</button
    >
    <button onclick={onclear} class="mc-secondary px-4 py-2.5 text-xs font-bold text-cyan-50"
      >Clear</button
    >
  </div>
</div>

<section class="mc-panel overflow-hidden">
  <div class="flex items-center gap-3 border-b border-cyan-300/10 bg-[#061329] px-4 py-3">
    <span class="size-2 {running ? 'bg-emerald-400 shadow-[0_0_8px_#34d399]' : 'bg-slate-500'}"
    ></span>
    <span class="text-[10px] font-bold uppercase tracking-[.14em] text-[#86aac8]"
      >{running ? "Minecraft running" : "Minecraft stopped"}</span
    >
    <label
      class="ml-auto flex items-center gap-2 text-[10px] uppercase tracking-wider text-[#6685a4]"
      ><input type="checkbox" bind:checked={follow} class="accent-cyan-400" /> Follow output</label
    >
    <span class="text-[10px] tabular-nums text-[#4f718f]"
      >{visibleLogs.length} / {logs.length} lines</span
    >
  </div>
  <div
    class="flex flex-wrap items-center gap-2 border-b border-cyan-300/10 bg-[#040e20] px-4 py-3"
    aria-label="Log categories"
  >
    {#each filters as item}
      <button
        onclick={() => (filter = item.id)}
        aria-pressed={filter === item.id}
        class="flex items-center gap-2 border px-3 py-1.5 text-[10px] font-bold uppercase tracking-[.08em] transition {filter ===
        item.id
          ? 'border-cyan-300/45 bg-cyan-400/15 text-cyan-50 shadow-[inset_2px_0_0_#19d8f1]'
          : 'border-cyan-300/10 bg-[#07162c] text-[#668aa9] hover:border-cyan-300/30 hover:text-cyan-100'}"
      >
        <span
          class="size-1.5 {item.id === 'error'
            ? 'bg-red-400'
            : item.id === 'warning'
              ? 'bg-amber-400'
              : item.id === 'debug'
                ? 'bg-violet-400'
                : item.id === 'info'
                  ? 'bg-cyan-400'
                  : 'bg-slate-300'}"
        ></span>
        {item.label}<span class="text-[9px] opacity-60">{counts[item.id]}</span>
      </button>
    {/each}
  </div>
  <div
    bind:this={consoleElement}
    class="h-[500px] overflow-auto bg-[#010714] p-4 font-mono text-[11px] leading-[1.65] selection:bg-cyan-400/25"
  >
    {#if visibleLogs.length}
      {#each visibleLogs as item}
        <div class="group flex border-l-2 px-3 py-px {levelClasses(item.level)}">
          <span class="mr-3 w-12 shrink-0 select-none text-[9px] font-bold uppercase opacity-45"
            >{item.level === "warning" ? "WARN" : item.level}</span
          >
          <span class="whitespace-pre-wrap break-all">{item.entry.line}</span>
        </div>
      {/each}
    {:else if logs.length}
      <div class="grid h-full place-items-center text-center text-[#4f718f]">
        <div>
          <div class="text-sm font-bold uppercase tracking-wider text-[#688dab]">
            No {filter} messages
          </div>
          <p class="mt-2 text-xs">Choose another category to see more output.</p>
        </div>
      </div>
    {:else}
      <div class="grid h-full place-items-center text-center text-[#4f718f]">
        <div>
          <div class="text-sm font-bold uppercase tracking-wider text-[#688dab]">
            Waiting for Minecraft
          </div>
          <p class="mt-2 text-xs">Game output will appear here after launch.</p>
        </div>
      </div>
    {/if}
  </div>
</section>
