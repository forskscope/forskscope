<script lang="ts">
  import Diff from '../../../components/diff/Diff.svelte'
  import Explorer from '../../../components/explorer/Explorer.svelte'
  import { compareSets, exploreIsActive } from '../../../stores/compareSets.svelte'
</script>

<!-- views -->

<!-- preserve locations w/ d-none instead of {#if} -->
<div class={`${exploreIsActive() ? '' : 'd-none'}`}>
  <Explorer />
</div>

{#each $compareSets as _, i}
  <Diff compareSetIndex={i} />
{/each}
