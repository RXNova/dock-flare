<script lang="ts">
  import { store } from '$lib/store.svelte';
  import { api } from '$lib/api';
  import TunnelRow from './TunnelRow.svelte';
  import TunnelForm from './TunnelForm.svelte';

  let project    = $derived(store.selectedProject!);
  let showForm   = $state(false);

  async function refresh() {
    store.tunnelsLoading = true;
    try {
      store.tunnels = await api.listProjectTunnels(project);
    } catch (e) {
      store.appendLog(`Error listing tunnels: ${e}`);
    } finally {
      store.tunnelsLoading = false;
    }
  }

  // Refresh whenever the selected project changes
  $effect(() => {
    const id = store.selectedId;
    if (id) refresh();
  });

  const authBadge = $derived(
    project.auth_mode === 'token' ? 'API token' : 'cloudflared'
  );

  // Return to AuthSetup to re-authenticate (browser projects only)
  async function reauth() {
    const updated = { ...project, browser_authed: false };
    await api.upsertProject(updated);
    store.upsertProject(updated);
  }
</script>

<div class="flex-1 overflow-y-auto p-5 space-y-3">
  <!-- Project header -->
  <div class="flex items-center justify-between">
    <div>
      <h2 class="text-sm font-semibold">{project.name || project.domain}</h2>
      <div class="flex items-center gap-1.5 mt-0.5">
        <span class="w-1.5 h-1.5 rounded-full bg-emerald-400"></span>
        {#if project.domain}
          <span class="text-[11px] font-mono text-base-content/50">{project.domain}</span>
          <span class="text-base-content/20">·</span>
        {/if}
        <span class="text-[11px] text-base-content/40">{authBadge}</span>
        {#if project.auth_mode === 'browser'}
          <span class="text-base-content/20">·</span>
          <button class="text-[11px] text-base-content/30 hover:text-base-content/60 transition-colors"
                  onclick={reauth} disabled={store.busy}>re-auth</button>
        {/if}
      </div>
    </div>
    <div class="flex items-center gap-2">
      <!-- Refresh -->
      <button
        class="btn btn-ghost btn-sm gap-1.5 text-base-content/40"
        onclick={refresh}
        disabled={store.busy || store.tunnelsLoading}
      >
        {#if store.tunnelsLoading}
          <span class="loading loading-spinner loading-xs"></span>
        {:else}
          <svg class="w-3.5 h-3.5" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor">
            <path fill-rule="evenodd" clip-rule="evenodd"
              d="M4.755 10.059a7.5 7.5 0 0112.548-3.364l1.903 1.927H15a.75.75 0 000 1.5h4.5a.75.75
                 0 00.75-.75V4.5a.75.75 0 00-1.5 0v3.093L17.53 5.887a9 9 0 00-15.044 4.034.75.75 0
                 001.269.138zM19.245 13.941a.75.75 0 00-1.269-.138A7.5 7.5 0 016.427 17.167l-1.903-1.927H9a.75.75
                 0 000-1.5H4.5a.75.75 0 00-.75.75v4.5a.75.75 0 001.5 0v-3.093l1.72 1.706a9 9 0 0015.044-4.034.75.75
                 0 00-1.769-.438z"/>
          </svg>
        {/if}
        Refresh
      </button>
      <!-- Add tunnel -->
      <button
        class="btn btn-primary btn-sm gap-1.5"
        onclick={() => (showForm = !showForm)}
        disabled={store.busy}
      >
        {#if showForm}
          Cancel
        {:else}
          <svg class="w-3.5 h-3.5" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor">
            <path fill-rule="evenodd" clip-rule="evenodd"
              d="M12 3.75a.75.75 0 01.75.75v6.75h6.75a.75.75 0 010 1.5h-6.75v6.75a.75.75 0 01-1.5
                 0v-6.75H4.5a.75.75 0 010-1.5h6.75V4.5a.75.75 0 01.75-.75z"/>
          </svg>
          Add Tunnel
        {/if}
      </button>
    </div>
  </div>

  <!-- Create form -->
  {#if showForm}
    <TunnelForm onSuccess={() => (showForm = false)} onCancel={() => (showForm = false)} />
  {/if}

  <!-- Tunnel table -->
  <div class="rounded-xl border border-base-300 bg-base-200 overflow-hidden">
    <!-- Table header -->
    <div class="flex items-center gap-3 px-4 py-2 border-b border-base-300 bg-base-300/30">
      <span class="w-2 flex-shrink-0"></span>
      <span class="text-[10px] font-semibold uppercase tracking-widest text-base-content/30 w-36">Name / Type</span>
      <span class="text-[10px] font-semibold uppercase tracking-widest text-base-content/30 flex-1">Hostname</span>
      <span class="text-[10px] font-semibold uppercase tracking-widest text-base-content/30 w-36 hidden xl:block">Service</span>
      <span class="text-[10px] font-semibold uppercase tracking-widest text-base-content/30 w-20">Namespace</span>
      <span class="text-[10px] font-semibold uppercase tracking-widest text-base-content/30 w-14 text-right">Status</span>
      <span class="w-24"></span>
    </div>

    {#if store.tunnelsLoading && store.tunnels.length === 0}
      <div class="flex items-center justify-center gap-2 py-8 text-base-content/30">
        <span class="loading loading-spinner loading-sm"></span>
        <span class="text-xs">Loading tunnels…</span>
      </div>
    {:else if store.tunnels.length === 0}
      <div class="flex flex-col items-center justify-center py-10 gap-2">
        <p class="text-sm text-base-content/30">No tunnels yet</p>
        <button
          class="btn btn-ghost btn-xs text-base-content/40 gap-1"
          onclick={() => (showForm = true)}
        >
          <svg class="w-3 h-3" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor">
            <path fill-rule="evenodd" clip-rule="evenodd"
              d="M12 3.75a.75.75 0 01.75.75v6.75h6.75a.75.75 0 010 1.5h-6.75v6.75a.75.75 0 01-1.5
                 0v-6.75H4.5a.75.75 0 010-1.5h6.75V4.5a.75.75 0 01.75-.75z"/>
          </svg>
          Add your first tunnel
        </button>
      </div>
    {:else}
      {#each store.tunnels as tunnel (tunnel.id)}
        <TunnelRow {tunnel} />
      {/each}
    {/if}
  </div>
</div>
