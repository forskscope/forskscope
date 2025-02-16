<script lang="ts">
  import type { DiffFilepaths } from '../../types'
  import Diff from '../diff/Diff.svelte'
  import Explorer from '../explorer/Explorer.svelte'

  const {
    diffFilepaths,
    addDiffTab,
  }: {
    diffFilepaths: DiffFilepaths | null
    addDiffTab: (diffFilepaths: DiffFilepaths) => void
  } = $props()

  const diffFilepathsOnSelected = (diffFilepaths: DiffFilepaths) => {
    addDiffTab(diffFilepaths)
  }
</script>

{#if diffFilepaths === null}
  <Explorer {diffFilepathsOnSelected} />
{:else}
  <Diff {diffFilepaths} />
{/if}
