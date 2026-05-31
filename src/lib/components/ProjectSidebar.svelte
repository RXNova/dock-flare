<script lang="ts">
  import { store } from '$lib/store.svelte';
  import { api } from '$lib/api';
  import type { Project } from '$lib/types';

  let showAdd   = $state(false);
  let newDomain = $state('');
  let adding    = $state(false);

  function isAuthenticated(p: Project): boolean {
    return p.auth_mode === 'token'
      ? !!(p.api_token && p.account_id)
      : p.browser_authed === true;
  }

  async function addProject() {
    const trimmed = newDomain.trim().toLowerCase();
    if (!trimmed) return;
    adding = true;
    const p: Project = {
      id: crypto.randomUUID(),
      domain: trimmed,
      auth_mode: 'token', // default; user picks in AuthSetup
      api_token: '',
      account_id: '',
    };
    await api.upsertProject(p);
    store.upsertProject(p);
    store.selectedId = p.id;
    newDomain = '';
    showAdd   = false;
    adding    = false;
  }

  async function removeProject(e: MouseEvent, id: string) {
    e.stopPropagation();
    await api.deleteProject(id);
    store.removeProject(id);
  }
</script>

<aside class="w-60 flex-shrink-0 border-r border-base-300 flex flex-col bg-base-200/40">
  <!-- Title -->
  <div class="px-4 pt-4 pb-2 flex items-center justify-between">
    <span class="text-[10px] font-semibold uppercase tracking-widest text-base-content/40">
      Projects
    </span>
    <button
      class="w-5 h-5 rounded flex items-center justify-center text-base-content/40
             hover:text-base-content/70 hover:bg-base-300 transition-colors"
      onclick={() => (showAdd = !showAdd)}
      title="Add project"
    >
      <svg class="w-3.5 h-3.5" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor">
        <path fill-rule="evenodd" clip-rule="evenodd"
          d="M12 3.75a.75.75 0 01.75.75v6.75h6.75a.75.75 0 010 1.5h-6.75v6.75a.75.75 0 01-1.5
             0v-6.75H4.5a.75.75 0 010-1.5h6.75V4.5a.75.75 0 01.75-.75z"/>
      </svg>
    </button>
  </div>

  <!-- Add project form -->
  {#if showAdd}
    <div class="mx-3 mb-2 rounded-lg border border-base-300 bg-base-100 p-3 space-y-2">
      <input
        type="text"
        placeholder="example.com"
        bind:value={newDomain}
        onkeydown={(e) => e.key === 'Enter' && addProject()}
        class="input input-xs w-full bg-base-200 border-base-300 font-mono
               focus:outline-none focus:ring-1 focus:ring-primary/30"
      />
      <div class="flex gap-1.5">
        <button
          class="btn btn-primary btn-xs flex-1"
          onclick={addProject}
          disabled={!newDomain.trim() || adding}
        >
          {#if adding}<span class="loading loading-spinner loading-xs"></span>{/if}
          Add
        </button>
        <button class="btn btn-ghost btn-xs" onclick={() => (showAdd = false)}>Cancel</button>
      </div>
    </div>
  {/if}

  <!-- Project list -->
  <div class="flex-1 overflow-y-auto space-y-0.5 px-2 pb-4">
    {#each store.projects as p (p.id)}
      {@const authed = isAuthenticated(p)}
      <!-- Using div+role to allow a real <button> inside for the delete action -->
      <div
        role="button"
        tabindex="0"
        class="w-full flex items-center gap-2.5 px-2 py-2 rounded-lg cursor-pointer transition-colors group
               {store.selectedId === p.id
                 ? 'bg-primary/10 border border-primary/20'
                 : 'hover:bg-base-300/60 border border-transparent'}
               {store.busy ? 'pointer-events-none opacity-60' : ''}"
        onclick={() => { store.selectedId = p.id; store.tunnels = []; }}
        onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { store.selectedId = p.id; store.tunnels = []; } }}
      >
        <!-- Auth status dot -->
        <span class="w-2 h-2 rounded-full flex-shrink-0 {authed ? 'bg-emerald-400' : 'bg-amber-400'}"></span>

        <!-- Domain -->
        <span class="text-xs font-medium flex-1 truncate
                     {store.selectedId === p.id ? 'text-base-content' : 'text-base-content/70'}">
          {p.domain}
        </span>

        <!-- Auth mode badge -->
        <span class="text-[9px] text-base-content/30 font-mono hidden group-hover:inline">
          {p.auth_mode === 'token' ? 'api' : 'cli'}
        </span>

        <!-- Remove -->
        <button
          class="w-4 h-4 rounded flex items-center justify-center text-base-content/20
                 hover:text-red-400 hover:bg-red-400/10 opacity-0 group-hover:opacity-100 transition-all"
          onclick={(e) => removeProject(e, p.id)}
          title="Remove project"
        >
          <svg class="w-2.5 h-2.5" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor">
            <path fill-rule="evenodd" clip-rule="evenodd"
              d="M5.47 5.47a.75.75 0 011.06 0L12 10.94l5.47-5.47a.75.75 0 111.06 1.06L13.06 12l5.47
                 5.47a.75.75 0 11-1.06 1.06L12 13.06l-5.47 5.47a.75.75 0 01-1.06-1.06L10.94 12 5.47
                 6.53a.75.75 0 010-1.06z"/>
          </svg>
        </button>
      </div>
    {:else}
      <p class="px-2 py-3 text-xs text-base-content/30 text-center">
        No projects yet.<br/>Click + to add a domain.
      </p>
    {/each}
  </div>
</aside>
