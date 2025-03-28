<script lang="ts">
  import Diff from '../../../components/diff/Diff.svelte'
  import Explorer from '../../../components/explorer/Explorer.svelte'
  import { activeCompareSet, exploreIsActive } from '../../../stores/tabs.svelte'
</script>

{#if exploreIsActive()}
  <Explorer />
{:else}
  {#key activeCompareSet()}
    <Diff />
  {/key}
{/if}
