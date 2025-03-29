<script lang="ts">
  import Diff from '../../../components/diff/Diff.svelte'
  import DiffRoot from '../../../components/diff/DiffRoot.svelte'
  import Explorer from '../../../components/explorer/Explorer.svelte'
  import {
    compareSets,
    exploreIsActive,
    isActiveCompareSetIndex,
  } from '../../../stores/tabs.svelte'
</script>

<!-- views -->

<!-- preserve locations w/ d-none instead of {#if} -->
<div class={`${exploreIsActive() ? '' : 'd-none'}`}>
  <Explorer />
</div>

{#each $compareSets as compareSet, i}
  <!-- preserve operations w/ d-none instead of {#if} -->
  <!-- <div class={`diff-wapper ${isActiveCompareSetIndex(i) ? '' : 'd-none'}`}> -->
  <!-- <Diff {compareSet} /> -->
  <!-- todo -->
  <DiffRoot {compareSet} visible={isActiveCompareSetIndex(i)} />
  <!-- </div> -->
{/each}
