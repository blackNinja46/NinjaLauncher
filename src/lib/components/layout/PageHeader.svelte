<script lang="ts">
  let { title, description }: { title: string; description: string } = $props();
</script>

<header class="mb-7">
  <div
    class="mb-2 h-1 w-12 bg-gradient-to-r from-cyan-300 to-blue-600 shadow-[0_0_12px_rgba(25,216,241,.5)]"
  ></div>
  <h1 class="block-title text-[25px] uppercase text-white">{title}</h1>
  <p class="mt-1 text-[13px] text-[#7595b7]">{description}</p>
</header>
