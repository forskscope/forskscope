<script lang="ts">
  import type { CompareSet } from '../../types'
  import Diff from '../diff/Diff.svelte'
  import Explorer from '../explorer/Explorer.svelte'

  const {
    compareSet,
    compareSetOnSelected,
    removeDiffTab,
  }: {
    compareSet: CompareSet | undefined
    compareSetOnSelected: (compareSet: CompareSet) => void
    removeDiffTab: () => void
  } = $props()
</script>

{#if !compareSet}
  <Explorer {compareSetOnSelected} />
{:else}
  <Diff {compareSet} close={removeDiffTab} />
{/if}
