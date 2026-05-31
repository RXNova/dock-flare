<script lang="ts">
  import { store } from '$lib/store.svelte';
  import { api } from '$lib/api';
  import type { Project } from '$lib/types';
  import { Plus, Pencil, X } from '@lucide/svelte';

  let showAdd    = $state(false);
  let newName    = $state('');
  let adding     = $state(false);
  let renamingId = $state<string | null>(null);
  let renameVal  = $state('');

  function isAuthenticated(p: Project): boolean {
    return p.auth_mode === 'token'
      ? !!(p.api_token && p.account_id)
      : p.browser_authed === true;
  }

  async function addProject() {
    const trimmed = newName.trim();
    if (!trimmed) return;
    adding = true;
    const p: Project = {
      id: crypto.randomUUID(),
      name: trimmed,
      domain: '',         // discovered from auth
      auth_mode: 'token', // default; user picks in AuthSetup
      api_token: '',
      account_id: '',
      browser_authed: false,
    };
    await api.upsertProject(p);
    store.upsertProject(p);
    store.selectedId = p.id;
    newName = '';
    showAdd = false;
    adding  = false;
  }

  async function removeProject(e: MouseEvent, id: string) {
    e.stopPropagation();
    await api.deleteProject(id);
    store.removeProject(id);
  }

  function startRename(e: MouseEvent, p: Project) {
    e.stopPropagation();
    renamingId = p.id;
    renameVal  = p.name;
  }

  async function commitRename(p: Project) {
    const trimmed = renameVal.trim();
    renamingId = null;
    if (!trimmed || trimmed === p.name) return;
    const updated = { ...p, name: trimmed };
    await api.upsertProject(updated);
    store.upsertProject(updated);
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
      <Plus class="w-3.5 h-3.5" strokeWidth={2} />
    </button>
  </div>

  <!-- Add project form -->
  {#if showAdd}
    <div class="mx-3 mb-2 rounded-lg border border-base-300 bg-base-100 p-3 space-y-2">
      <input
        type="text"
        placeholder="Project name"
        bind:value={newName}
        onkeydown={(e) => e.key === 'Enter' && addProject()}
        class="input input-xs w-full bg-base-200 border-base-300
               focus:outline-none focus:ring-1 focus:ring-primary/30"
      />
      <p class="text-[10px] text-base-content/30 px-0.5">
        The domain is detected automatically when you authenticate.
      </p>
      <div class="flex gap-1.5">
        <button
          class="btn btn-primary btn-xs flex-1"
          onclick={addProject}
          disabled={!newName.trim() || adding}
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

        <!-- Name + discovered domain (or inline rename input) -->
        {#if renamingId === p.id}
          <input
            type="text"
            bind:value={renameVal}
            onkeydown={(e) => { if (e.key === 'Enter') commitRename(p); else if (e.key === 'Escape') renamingId = null; }}
            onblur={() => commitRename(p)}
            onclick={(e) => e.stopPropagation()}
            class="flex-1 min-w-0 input input-xs bg-base-100 border-base-300 text-xs font-medium
                   focus:outline-none focus:ring-1 focus:ring-primary/40"
          />
        {:else}
          <span class="flex-1 min-w-0">
            <span class="block text-xs font-medium truncate
                         {store.selectedId === p.id ? 'text-base-content' : 'text-base-content/70'}">
              {p.name || 'Untitled'}
            </span>
            {#if p.domain}
              <span class="block text-[10px] text-base-content/35 font-mono truncate">{p.domain}</span>
            {/if}
          </span>

          <!-- Rename -->
          <button
            class="w-4 h-4 rounded flex items-center justify-center text-base-content/20
                   hover:text-base-content/60 hover:bg-base-300 opacity-0 group-hover:opacity-100 transition-all"
            onclick={(e) => startRename(e, p)}
            title="Rename project"
          >
            <Pencil class="w-2.5 h-2.5" strokeWidth={2} />
          </button>

          <!-- Remove -->
          <button
            class="w-4 h-4 rounded flex items-center justify-center text-base-content/20
                   hover:text-red-400 hover:bg-red-400/10 opacity-0 group-hover:opacity-100 transition-all"
            onclick={(e) => removeProject(e, p.id)}
            title="Remove project"
          >
            <X class="w-2.5 h-2.5" strokeWidth={2} />
          </button>
        {/if}
      </div>
    {:else}
      <p class="px-2 py-3 text-xs text-base-content/30 text-center">
        No projects yet.<br/>Click + to add one.
      </p>
    {/each}
  </div>
</aside>
