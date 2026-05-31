<script lang="ts">
  import { store } from '$lib/store.svelte';
  import { lineStyle } from '$lib/utils';

  let logEl = $state<HTMLDivElement | null>(null);

  $effect(() => {
    store.logs; // track changes
    setTimeout(() => { if (logEl) logEl.scrollTop = logEl.scrollHeight; }, 0);
  });
</script>

<div class="flex-shrink-0 border-t border-zinc-800 bg-zinc-950">
  <div class="flex items-center gap-2 px-3 py-2 border-b border-zinc-800/60">
    <span class="w-2.5 h-2.5 rounded-full bg-zinc-700"></span>
    <span class="w-2.5 h-2.5 rounded-full bg-zinc-700"></span>
    <span class="w-2.5 h-2.5 rounded-full bg-zinc-700"></span>
    <span class="ml-2 text-[10px] font-mono text-zinc-500">deployment log</span>
    {#if store.logs.length > 0}
      <button
        class="ml-auto text-[10px] text-zinc-600 hover:text-zinc-300 transition-colors"
        onclick={() => store.clearLogs()}
      >clear</button>
    {/if}
  </div>
  <div
    class="p-3 h-36 overflow-y-auto font-mono text-[11px] leading-[1.6] select-text cursor-text"
    bind:this={logEl}
  >
    {#if store.logs.length === 0}
      <span class="text-zinc-500">ready</span><span class="text-zinc-500 cursor-blink">█</span>
    {:else}
      {#each store.logs as line}
        <div class={lineStyle(line)}>{line}</div>
      {/each}
    {/if}
  </div>
</div>
