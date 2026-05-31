<script lang="ts">
  import { store } from '$lib/store.svelte';
  import { api } from '$lib/api';
  import type { TunnelInfo } from '$lib/types';
  import { Check, Copy, ExternalLink, Pencil, Trash2, X } from '@lucide/svelte';

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

  <!-- Hostname + copy + open button -->
  <div class="flex items-center gap-1 flex-1 min-w-0">
    <span class="text-xs text-base-content/50 truncate font-mono">{tunnel.hostname ?? '—'}</span>
    {#if tunnel.hostname}
      <button
        class="opacity-0 group-hover:opacity-60 hover:!opacity-100 transition-opacity flex-shrink-0"
        onclick={copyHostname}
        title={copied ? 'Copied!' : 'Copy hostname'}
      >
        {#if copied}
          <Check class="w-3 h-3 text-emerald-400" strokeWidth={2.5} />
        {:else}
          <Copy class="w-3 h-3 text-base-content/50" strokeWidth={2} />
        {/if}
      </button>
      <button
        class="opacity-0 group-hover:opacity-60 hover:!opacity-100 transition-opacity flex-shrink-0"
        onclick={() => api.openUrl('https://' + tunnel.hostname)}
        title="Open in browser"
      >
        <ExternalLink class="w-3 h-3 text-base-content/50" strokeWidth={2} />
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
        {:else}
          <Check class="w-3 h-3" strokeWidth={2.5} />
        {/if}
      </button>
      <button
        class="btn btn-ghost btn-xs px-1.5 text-base-content/40"
        onclick={() => (confirming = false)}
        disabled={store.busy}
      >
        <X class="w-3 h-3" strokeWidth={2} />
      </button>
    {:else}
      <!-- Edit service button (token mode) -->
      {#if store.selectedProject?.auth_mode === 'token'}
        <button
          class="btn btn-ghost btn-xs opacity-0 group-hover:opacity-100 transition-opacity
                 text-base-content/40 hover:text-primary hover:bg-primary/10"
          onclick={() => (editing = true)}
          disabled={store.busy}
          title="Edit internal service URL"
        >
          <Pencil class="w-3 h-3" strokeWidth={2} />
        </button>
      {/if}
      <button
        class="btn btn-ghost btn-xs opacity-0 group-hover:opacity-100 transition-opacity
               text-base-content/40 hover:text-red-400 hover:bg-red-400/10 gap-1"
        onclick={() => (confirming = true)}
        disabled={store.busy}
      >
        <Trash2 class="w-3 h-3" strokeWidth={2} />
        Tear down
      </button>
    {/if}
  </div>
</div>
