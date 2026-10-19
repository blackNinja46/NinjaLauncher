<script lang="ts">
  import Icon from "./Icon.svelte";

  let {
    value = $bindable(),
    min = 1024,
    max,
    step = 256,
    disabled = false,
  }: {
    value: number;
    min?: number;
    max: number;
    step?: number;
    disabled?: boolean;
  } = $props();

  let progress = $derived(
    max <= min ? 0 : Math.max(0, Math.min(100, ((value - min) / (max - min)) * 100)),
  );

  function formatMemory(megabytes: number): string {
    const gigabytes = megabytes / 1024;
    return `${Number.isInteger(gigabytes) ? gigabytes : gigabytes.toFixed(1)} GB`;
  }

  function clamp(next: number): number {
    if (!Number.isFinite(next)) return value;
    return Math.max(min, Math.min(max, Math.round(next / step) * step));
  }

  function update(next: number) {
    value = clamp(next);
  }
</script>

<div class="memory-range" class:memory-range-disabled={disabled}>
  <div class="memory-range-summary">
    <div>
      <span class="memory-range-eyebrow">Game memory</span>
      <strong>{formatMemory(value)}</strong>
    </div>
    <label class="memory-range-input">
      <span class="sr-only">Allocated memory in megabytes</span>
      <input
        type="number"
        {min}
        {max}
        {step}
        {disabled}
        {value}
        oninput={(event) => {
          const next = event.currentTarget.valueAsNumber;
          if (Number.isFinite(next)) value = next;
        }}
        onblur={(event) => update(event.currentTarget.valueAsNumber)}
        onkeydown={(event) => {
          if (event.key === "Enter") event.currentTarget.blur();
        }}
      />
      <span>MB</span>
    </label>
  </div>

  <div class="memory-range-controls">
    <button
      type="button"
      aria-label="Decrease allocated memory"
      disabled={disabled || value <= min}
      onclick={() => update(value - step)}
    >
      <Icon name="minus" size={10} />
    </button>
    <div class="memory-range-track" style={`--memory-progress: ${progress}%`}>
      <input
        type="range"
        {min}
        {max}
        {step}
        {value}
        {disabled}
        aria-label="Allocated memory"
        oninput={(event) => update(event.currentTarget.valueAsNumber)}
      />
    </div>
    <button
      type="button"
      aria-label="Increase allocated memory"
      disabled={disabled || value >= max}
      onclick={() => update(value + step)}
    >
      <Icon name="plus" size={10} />
    </button>
  </div>

  <div class="memory-range-limits" aria-hidden="true">
    <span>{formatMemory(min)} minimum</span>
    <span>{formatMemory(max)} available</span>
  </div>
</div>

<style>
  .memory-range {
    width: min(30rem, 48vw);
    min-width: 18rem;
    padding: 0.8rem 0.9rem 0.65rem;
    border: 1px solid var(--color-border-default);
    border-radius: 9px;
    background:
      linear-gradient(
        120deg,
        color-mix(in srgb, var(--color-primary) 8%, transparent),
        transparent 48%
      ),
      var(--color-surface-field);
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.06),
      0 8px 20px rgba(0, 0, 0, 0.11);
  }

  .memory-range-summary,
  .memory-range-controls,
  .memory-range-limits {
    display: flex;
    align-items: center;
  }

  .memory-range-summary {
    justify-content: space-between;
    gap: 1rem;
    margin-bottom: 0.8rem;
  }

  .memory-range-summary > div {
    display: flex;
    align-items: baseline;
    gap: 0.55rem;
  }

  .memory-range-eyebrow {
    color: var(--color-text-faint);
    font-size: 0.56rem;
    font-weight: 800;
    letter-spacing: 0.12em;
    text-transform: uppercase;
  }

  strong {
    color: var(--color-text-primary);
    font-size: 0.85rem;
    letter-spacing: -0.02em;
  }

  .memory-range-input {
    display: flex;
    align-items: center;
    overflow: hidden;
    border: 1px solid var(--color-border-default);
    border-radius: 6px;
    background: color-mix(in srgb, var(--color-background) 72%, transparent);
    color: var(--color-text-faint);
    transition:
      border-color 160ms ease,
      box-shadow 160ms ease;
  }

  .memory-range-input:focus-within {
    border-color: var(--color-border-brand);
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--color-primary) 18%, transparent);
  }

  .memory-range-input input {
    width: 4.1rem;
    padding: 0.35rem 0.2rem 0.35rem 0.55rem;
    border: 0;
    outline: 0;
    background: transparent;
    color: var(--color-text-primary);
    font-size: 0.68rem;
    font-weight: 750;
    text-align: right;
    appearance: textfield;
  }

  .memory-range-input input::-webkit-inner-spin-button,
  .memory-range-input input::-webkit-outer-spin-button {
    margin: 0;
    appearance: none;
  }

  .memory-range-input span:last-child {
    padding-right: 0.5rem;
    font-size: 0.55rem;
    font-weight: 800;
  }

  .memory-range-controls {
    gap: 0.65rem;
  }

  .memory-range-controls button {
    display: grid;
    width: 1.65rem;
    height: 1.65rem;
    flex: 0 0 auto;
    place-items: center;
    border: 1px solid var(--color-border-default);
    border-radius: 6px;
    background: var(--color-surface-subtle);
    color: var(--color-text-muted);
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.07);
    transition: 150ms ease;
  }

  .memory-range-controls button:hover:not(:disabled) {
    border-color: var(--color-border-brand);
    color: var(--color-primary);
    transform: translateY(-1px);
  }

  .memory-range-controls button:disabled {
    cursor: default;
    opacity: 0.35;
  }

  .memory-range-track {
    position: relative;
    height: 1.65rem;
    flex: 1;
  }

  .memory-range-track::before {
    position: absolute;
    top: 50%;
    right: 0;
    left: 0;
    height: 0.32rem;
    border-radius: 999px;
    background:
      linear-gradient(
        90deg,
        var(--color-primary) var(--memory-progress),
        transparent var(--memory-progress)
      ),
      color-mix(in srgb, var(--color-text-primary) 10%, transparent);
    box-shadow:
      inset 0 1px 2px rgba(0, 0, 0, 0.35),
      0 0 12px color-mix(in srgb, var(--color-primary) 15%, transparent);
    content: "";
    transform: translateY(-50%);
  }

  .memory-range-track input {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    margin: 0;
    cursor: pointer;
    opacity: 0;
  }

  .memory-range-track::after {
    position: absolute;
    top: 50%;
    left: var(--memory-progress);
    width: 0.9rem;
    height: 0.9rem;
    border: 2px solid var(--color-text-on-brand);
    border-radius: 50%;
    background: var(--color-primary);
    box-shadow:
      0 0 0 3px color-mix(in srgb, var(--color-primary) 17%, transparent),
      0 3px 9px rgba(0, 0, 0, 0.35);
    content: "";
    pointer-events: none;
    transform: translate(-50%, -50%);
  }

  .memory-range-limits {
    justify-content: space-between;
    padding: 0.45rem 2.3rem 0;
    color: var(--color-text-faint);
    font-size: 0.52rem;
    font-weight: 650;
    letter-spacing: 0.03em;
  }

  .memory-range-disabled {
    opacity: 0.55;
  }

  @media (max-width: 800px) {
    .memory-range {
      width: 100%;
      min-width: 0;
    }
  }
</style>
