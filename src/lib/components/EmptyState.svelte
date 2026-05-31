<script lang="ts">
  import { Cloud } from '@lucide/svelte';
</script>

<div class="flex-1 flex items-center justify-center">
  <div class="text-center space-y-2">
    <div class="w-12 h-12 rounded-2xl bg-base-300/60 flex items-center justify-center mx-auto">
      <Cloud class="w-6 h-6 text-base-content/20" strokeWidth={1.5} />
    </div>
    <p class="text-sm font-medium text-base-content/40">No project selected</p>
    <p class="text-xs text-base-content/25">Add a domain in the sidebar to get started</p>
  </div>
</div>
