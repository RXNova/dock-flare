<script lang="ts">
  import { store } from '$lib/store.svelte';
  import { statusCfg } from '$lib/utils';
</script>

<header class="flex-shrink-0 px-5 py-3.5 border-b border-base-300
               flex items-center justify-between">
  <div class="flex items-center gap-3">
    <div class="w-8 h-8 rounded-lg bg-primary/10 flex items-center justify-center flex-shrink-0">
      <svg class="w-4 h-4 text-primary" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor">
        <path d="M4.5 9.75a6 6 0 0111.573-2.226 3.75 3.75 0 014.133 4.303A4.5 4.5 0
                 0118 20.25H6.75a5.25 5.25 0 01-2.23-10.004 6.072 6.072 0 01-.02-.496z"/>
      </svg>
    </div>
    <div>
      <p class="text-sm font-semibold leading-none tracking-tight">DockFlare</p>
      <p class="text-[11px] text-base-content/35 mt-0.5">Cloudflare tunnels → local Kubernetes</p>
    </div>
  </div>

  <div class="flex items-center gap-2">
    <!-- Status pill -->
    <div class="flex items-center gap-2 px-3 py-1.5 rounded-full bg-base-200 border border-base-300">
      <span class="relative flex h-2 w-2">
        {#if store.status === 'processing'}
          <span class="animate-ping absolute inline-flex h-full w-full rounded-full opacity-60
                       {statusCfg.processing.dot}"></span>
        {/if}
        <span class="relative inline-flex h-2 w-2 rounded-full {statusCfg[store.status].dot}"></span>
      </span>
      <span class="text-[11px] font-medium {statusCfg[store.status].text}">
        {statusCfg[store.status].label}
      </span>
    </div>

    <!-- Theme toggle -->
    <button
      class="w-7 h-7 rounded-lg flex items-center justify-center
             text-base-content/40 hover:text-base-content/70 hover:bg-base-200 transition-colors"
      onclick={() => store.toggleTheme()}
    >
      {#if store.theme === 'dark'}
        <svg class="w-3.5 h-3.5" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor">
          <path d="M12 2.25a.75.75 0 01.75.75v2.25a.75.75 0 01-1.5 0V3a.75.75 0 01.75-.75zM7.5 12a4.5 4.5 0
                   0 119 0 4.5 4.5 0 01-9 0zM18.894 6.166a.75.75 0 00-1.06-1.06l-1.591 1.59a.75.75 0
                   0 101.06 1.061l1.591-1.59zM21.75 12a.75.75 0 01-.75.75h-2.25a.75.75 0 010-1.5H21a.75.75
                   0 01.75.75zM17.834 18.894a.75.75 0 001.06-1.06l-1.59-1.591a.75.75 0 10-1.061 1.06l1.59
                   1.591zM12 18a.75.75 0 01.75.75V21a.75.75 0 01-1.5 0v-2.25A.75.75 0 0112 18zM7.166
                   17.834a.75.75 0 00-1.06 1.06l1.59 1.591a.75.75 0 001.061-1.06l-1.59-1.591zM6 12a.75.75
                   0 01-.75.75H3a.75.75 0 010-1.5h2.25A.75.75 0 016 12zM6.166 6.166a.75.75 0 001.06-1.06
                   L5.634 3.515a.75.75 0 10-1.06 1.06l1.59 1.591z"/>
        </svg>
      {:else}
        <svg class="w-3.5 h-3.5" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor">
          <path fill-rule="evenodd" clip-rule="evenodd"
            d="M9.528 1.718a.75.75 0 01.162.819A8.97 8.97 0 009 6a9 9 0 009 9 8.97 8.97 0 003.463-.69.75.75
               0 01.981.98 10.503 10.503 0 01-9.694 6.46c-5.799 0-10.5-4.701-10.5-10.5 0-4.368 2.667-8.112
               6.46-9.694a.75.75 0 01.818.162z"/>
        </svg>
      {/if}
    </button>
  </div>
</header>
