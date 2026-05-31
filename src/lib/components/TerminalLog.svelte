<script lang="ts">
  import { store } from '$lib/store.svelte';
  import { api } from '$lib/api';
  import { lineStyle } from '$lib/utils';

  let logEl = $state<HTMLDivElement | null>(null);

  $effect(() => {
    store.logs;
    setTimeout(() => { if (logEl) logEl.scrollTop = logEl.scrollHeight; }, 0);
  });

  // Split a log line into text and URL segments so URLs can be rendered as links
  function segments(line: string): { text: string; url?: string }[] {
    const urlRe = /https?:\/\/[^\s]+/g;
    const parts: { text: string; url?: string }[] = [];
    let last = 0;
    let m: RegExpExecArray | null;
    while ((m = urlRe.exec(line)) !== null) {
      if (m.index > last) parts.push({ text: line.slice(last, m.index) });
      parts.push({ text: m[0], url: m[0] });
      last = m.index + m[0].length;
    }
    if (last < line.length) parts.push({ text: line.slice(last) });
    return parts.length ? parts : [{ text: line }];
  }
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
        <div class={lineStyle(line)}>
          {#each segments(line) as seg}
            {#if seg.url}
              <button
                class="underline decoration-dotted hover:decoration-solid text-sky-400 hover:text-sky-300
                       cursor-pointer transition-colors"
                onclick={() => api.openUrl(seg.url!)}
                title="Click to open in browser"
              >{seg.text}</button>
            {:else}
              {seg.text}
            {/if}
          {/each}
        </div>
      {/each}
    {/if}
  </div>
</div>
