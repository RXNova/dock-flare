<script lang="ts">
  import { store } from '$lib/store.svelte';
  import EmptyState from './EmptyState.svelte';
  import AuthSetup from './AuthSetup.svelte';
  import TunnelList from './TunnelList.svelte';
  import type { Project } from '$lib/types';

  function isAuthenticated(p: Project): boolean {
    return p.auth_mode === 'token'
      ? !!(p.api_token && p.account_id)
      : (store.cfAuthorized && store.cloudflaredFound === true);
  }

  let project    = $derived(store.selectedProject);
  let authed     = $derived(project ? isAuthenticated(project) : false);
</script>

<div class="flex-1 flex flex-col min-h-0">
  {#if !project}
    <EmptyState />
  {:else if !authed}
    <AuthSetup />
  {:else}
    <TunnelList />
  {/if}
</div>
