<script lang="ts">
  import Diff from '../../../components/diff/Diff.svelte'
  import Explorer from '../../../components/explorer/Explorer.svelte'
  import { activeTabIndex, EXPLORER_TAB_INDEX } from '../../../stores/tabs.svelte'
</script>

{#if $activeTabIndex === EXPLORER_TAB_INDEX}
  <Explorer />
{:else}
  <Diff />
{/if}
