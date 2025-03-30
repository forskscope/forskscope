<script lang="ts">
  import { Eye, ScanEye } from 'lucide-svelte'

  const {
    showsHumanReadableSize,
    toggleShowsHumanReadableSize,
  }: {
    showsHumanReadableSize: boolean
    toggleShowsHumanReadableSize: () => void
  } = $props()
</script>

<button onclick={toggleShowsHumanReadableSize}>
  {#if showsHumanReadableSize}
    <Eye />
  {:else}
    <ScanEye />
  {/if}
</button>

<style>
  button {
    padding: 0.2rem 0.4rem;
    font-size: 1.2rem;
  }
</style>
