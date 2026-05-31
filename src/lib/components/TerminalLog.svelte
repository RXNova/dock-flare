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

<div class="flex-shrink-0 border-t border-base-300 bg-base-100">
  <div class="flex items-center gap-2 px-3 py-2 border-b border-base-300 bg-base-200/50">
    <span class="w-2.5 h-2.5 rounded-full bg-base-content/20"></span>
    <span class="w-2.5 h-2.5 rounded-full bg-base-content/20"></span>
    <span class="w-2.5 h-2.5 rounded-full bg-base-content/20"></span>
    <span class="ml-2 text-[10px] font-mono text-base-content/50">deployment log</span>
    <div class="ml-auto flex items-center gap-3">
      <button
        class="text-[10px] text-base-content/40 hover:text-base-content/70 transition-colors"
        onclick={async () => { const p = await api.getLogFilePath(); api.openUrl('file://' + p); }}
        title="Open persistent log file"
      >view file</button>
      <button
        class="text-[10px] transition-colors
               {store.logs.length > 0 ? 'text-base-content/50 hover:text-base-content cursor-pointer' : 'text-base-content/20 cursor-default pointer-events-none'}"
        onclick={() => { store.clearLogs(); api.clearPersistentLogs(); }}
        title="Clear log file"
      >clear</button>
    </div>
  </div>
  <div
    class="p-3 h-36 overflow-y-auto font-mono text-[11px] leading-[1.6] select-text cursor-text bg-base-100"
    bind:this={logEl}
  >
    {#if store.logs.length === 0}
      <span class="text-base-content/50">ready</span><span class="text-base-content/50 cursor-blink">█</span>
    {:else}
      {#each store.logs as line}
        <div class={lineStyle(line)}>
          {#each segments(line) as seg}
            {#if seg.url}
              <button
                class="underline decoration-dotted hover:decoration-solid text-info hover:text-info-content
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
