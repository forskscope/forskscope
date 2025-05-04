<script lang="ts">
  import { Eye, ScanEye } from 'lucide-svelte'
  import Tooltip from '../../common/Tooltip.svelte'
  import { T } from '../../../stores/settings/translation.svelte'

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
    <Tooltip position="top" messages={T('Detailed information')}>
      <Eye />
    </Tooltip>
  {:else}
    <Tooltip position="top" messages={T('Normal information')}>
      <ScanEye />
    </Tooltip>
  {/if}
</button>

<style>
  button {
    padding: 0.2rem 0.4rem;
    font-size: 1.2rem;
  }
</style>
