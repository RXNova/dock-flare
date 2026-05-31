<script lang="ts">
  import { store } from '$lib/store.svelte';
  import { api } from '$lib/api';
  import type { TunnelInfo } from '$lib/types';

  let { tunnel }: { tunnel: TunnelInfo } = $props();

  let confirming     = $state(false);
  let pendingNs      = $state('');

  // Pre-fill namespace when confirm opens
  $effect(() => {
    if (confirming) pendingNs = tunnel.namespace ?? 'default';
  });

  async function teardown() {
    const project = store.selectedProject!;
    confirming = false;
    store.status = 'processing';
    store.clearLogs();
    try {
      await api.teardownTunnel(project, tunnel.name, tunnel.hostname ?? '', pendingNs);
      store.status = 'success';
      store.tunnels = await api.listProjectTunnels(project);
    } catch (e) {
      store.status = 'failed';
      store.appendLog(`FATAL: ${e}`);
    }
  }

  const statusDot: Record<string, string> = {
    healthy:  'bg-emerald-400',
    degraded: 'bg-amber-400',
    down:     'bg-red-400',
    inactive: 'bg-zinc-500',
    unknown:  'bg-zinc-600',
    active:   'bg-emerald-400',
  };
  const dot = $derived(statusDot[tunnel.status] ?? 'bg-zinc-600');
</script>

<div class="flex items-center gap-3 px-4 py-2.5 border-b border-base-300/60 last:border-0
            hover:bg-base-300/20 transition-colors group">
  <!-- Status dot -->
  <span class="w-2 h-2 rounded-full flex-shrink-0 {dot}"></span>

  <!-- Name + type badge -->
  <div class="flex items-center gap-1.5 w-36 min-w-0">
    <span class="text-xs font-medium truncate">{tunnel.name}</span>
    {#if tunnel.target_type}
      <span class="badge badge-sm flex-shrink-0 {tunnel.target_type === 'local' ? 'badge-info' : 'badge-warning'} opacity-70">
        {tunnel.target_type}
      </span>
    {/if}
  </div>

  <!-- Hostname -->
  <span class="text-xs text-base-content/50 flex-1 truncate font-mono">
    {tunnel.hostname ?? '—'}
  </span>

  <!-- Service -->
  <span class="text-xs text-base-content/40 w-36 truncate font-mono hidden xl:block">
    {tunnel.service ?? '—'}
  </span>

  <!-- Namespace -->
  <span class="text-[10px] text-base-content/30 font-mono w-20 truncate">
    {tunnel.namespace ?? '—'}
  </span>

  <!-- Status label -->
  <span class="text-[10px] w-14 text-right
               {tunnel.status === 'healthy' ? 'text-emerald-400' : 'text-zinc-400'}">
    {tunnel.status}
  </span>

  <!-- Actions -->
  <div class="w-24 flex items-center justify-end gap-1">
    {#if confirming}
      <!-- Namespace editable + confirm -->
      <input
        type="text"
        bind:value={pendingNs}
        class="input input-xs w-20 bg-base-100 border-base-300 font-mono text-[10px]
               focus:outline-none focus:ring-1 focus:ring-red-500/30"
        placeholder="namespace"
        title="K8s namespace to delete"
      />
      <button
        class="btn btn-error btn-xs px-2"
        onclick={teardown}
        disabled={store.busy}
        title="Confirm teardown"
      >
        {#if store.busy}
          <span class="loading loading-spinner loading-xs"></span>
        {:else}✓{/if}
      </button>
      <button
        class="btn btn-ghost btn-xs px-1.5 text-base-content/40"
        onclick={() => (confirming = false)}
        disabled={store.busy}
      >✕</button>
    {:else}
      <button
        class="btn btn-ghost btn-xs opacity-0 group-hover:opacity-100 transition-opacity
               text-base-content/40 hover:text-red-400 hover:bg-red-400/10 gap-1"
        onclick={() => (confirming = true)}
        disabled={store.busy}
      >
        <svg class="w-3 h-3" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor">
          <path fill-rule="evenodd" clip-rule="evenodd"
            d="M16.5 4.478v.227a48.816 48.816 0 013.878.512.75.75 0 11-.256 1.478l-.209-.035-1.005
               13.07a3 3 0 01-2.991 2.77H8.084a3 3 0 01-2.991-2.77L4.087 6.66l-.209.035a.75.75
               0 01-.256-1.478A48.567 48.567 0 017.5 4.705v-.227c0-1.564 1.213-2.9 2.816-2.951a52.662
               52.662 0 013.369 0c1.603.051 2.815 1.387 2.815 2.951zm-6.136-1.452a51.196 51.196 0
               013.273 0C14.39 3.05 15 3.684 15 4.478v.113a49.488 49.488 0 00-6 0v-.113c0-.794.609-1.428
               1.364-1.452zm-.355 5.945a.75.75 0 10-1.5.058l.347 9a.75.75 0 101.499-.058l-.346-9zm5.48.058a.75.75
               0 10-1.498-.058l-.347 9a.75.75 0 001.5.058l.345-9z"/>
        </svg>
        Tear down
      </button>
    {/if}
  </div>
</div>
