<script lang="ts">
  import { store } from '$lib/store.svelte';
  import { api } from '$lib/api';
  import type { TunnelInfo } from '$lib/types';

  let { tunnel }: { tunnel: TunnelInfo } = $props();

  let confirming     = $state(false);
  let pendingNs      = $state('');
  let editing        = $state(false);
  let editService    = $state('');
  let copied         = $state(false);

  function copyHostname() {
    navigator.clipboard.writeText(tunnel.hostname ?? '');
    copied = true;
    setTimeout(() => { copied = false; }, 1500);
  }

  $effect(() => { if (editing) editService = tunnel.service ?? ''; });

  async function saveEdit() {
    const project = store.selectedProject!;
    try {
      await api.updateTunnelService(project, tunnel.name, editService);
      editing = false;
      store.tunnels = await api.listProjectTunnels(project);
    } catch (e) {
      store.appendLog(`Edit failed: ${e}`);
    }
  }

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

  <!-- Hostname + open button -->
  <div class="flex items-center gap-1 flex-1 min-w-0">
    <span class="text-xs text-base-content/50 truncate font-mono">{tunnel.hostname ?? '—'}</span>
    {#if tunnel.hostname}
      <button
        class="opacity-0 group-hover:opacity-60 hover:!opacity-100 transition-opacity flex-shrink-0 relative"
        onclick={copyHostname}
        title={copied ? 'Copied!' : 'Copy hostname'}
      >
        {#if copied}
          <svg class="w-3 h-3 text-emerald-400" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor">
            <path fill-rule="evenodd" clip-rule="evenodd"
              d="M19.916 4.626a.75.75 0 01.208 1.04l-9 13.5a.75.75 0 01-1.154.114l-6-6a.75.75
                 0 011.06-1.06l5.353 5.353 8.493-12.739a.75.75 0 011.04-.208z"/>
          </svg>
        {:else}
          <svg class="w-3 h-3 text-base-content/50" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor">
            <path fill-rule="evenodd" clip-rule="evenodd"
              d="M17.663 3.118c.225.015.45.032.673.05C19.876 3.298 21 4.604 21 6.109v9.642a3 3 0
                 01-3 3V16.5c0-5.922-4.576-10.775-10.384-11.217.324-1.132 1.3-2.01 2.548-2.114.224-.019.448-.036.673-.051A3
                 3 0 0113.5 1.5H15a3 3 0 012.663 1.618zM12 4.5A1.5 1.5 0 0113.5 3H15a1.5 1.5 0 011.5 1.5H12z"/>
            <path d="M3 8.625c0-1.036.84-1.875 1.875-1.875h.375A3.75 3.75 0 019 10.5v1.875c0 1.036.84
                     1.875 1.875 1.875h1.875A3.75 3.75 0 0116.5 18v2.625c0 1.035-.84 1.875-1.875 1.875h-9.75A1.875
                     1.875 0 013 20.625v-12z"/>
          </svg>
        {/if}
      </button>
      <button
        class="opacity-0 group-hover:opacity-60 hover:!opacity-100 transition-opacity flex-shrink-0"
        onclick={() => api.openUrl('https://' + tunnel.hostname)}
        title="Open in browser"
      >
        <svg class="w-3 h-3 text-base-content/50" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor">
          <path fill-rule="evenodd" clip-rule="evenodd"
            d="M15.75 2.25H21a.75.75 0 01.75.75v5.25a.75.75 0 01-1.5 0V4.81L8.03 17.03a.75.75 0
               01-1.06-1.06L19.19 3.75h-3.44a.75.75 0 010-1.5zm-10.5 4.5a1.5 1.5 0 00-1.5 1.5v10.5a1.5
               1.5 0 001.5 1.5h10.5a1.5 1.5 0 001.5-1.5V10.5a.75.75 0 011.5 0v8.25a3 3 0 01-3 3H5.25a3
               3 0 01-3-3V8.25a3 3 0 013-3h8.25a.75.75 0 010 1.5H5.25z"/>
        </svg>
      </button>
    {/if}
  </div>

  <!-- Service (editable) -->
  {#if editing}
    <div class="flex items-center gap-1 w-36">
      <input type="text" bind:value={editService}
        class="input input-xs w-full bg-base-100 border-base-300 font-mono text-[10px]
               focus:outline-none focus:ring-1 focus:ring-primary/30" />
      <button class="btn btn-xs btn-primary px-1.5" onclick={saveEdit}>✓</button>
      <button class="btn btn-ghost btn-xs px-1" onclick={() => (editing = false)}>✕</button>
    </div>
  {:else}
    <span class="text-xs text-base-content/40 w-36 truncate font-mono">
      {tunnel.service ?? '—'}
    </span>
  {/if}

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
      <!-- Edit service button (xl only, token mode) -->
      {#if store.selectedProject?.auth_mode === 'token'}
        <button
          class="btn btn-ghost btn-xs opacity-0 group-hover:opacity-100 transition-opacity
                 text-base-content/40 hover:text-primary hover:bg-primary/10"
          onclick={() => (editing = true)}
          disabled={store.busy}
          title="Edit internal service URL"
        >
          <svg class="w-3 h-3" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor">
            <path d="M21.731 2.269a2.625 2.625 0 00-3.712 0l-1.157 1.157 3.712 3.712 1.157-1.157a2.625
                     2.625 0 000-3.712zM19.513 8.199l-3.712-3.712-8.4 8.4a5.25 5.25 0 00-1.32 2.214l-.8
                     2.685a.75.75 0 00.933.933l2.685-.8a5.25 5.25 0 002.214-1.32l8.4-8.4z"/>
          </svg>
        </button>
      {/if}
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
