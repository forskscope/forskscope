<script lang="ts">
  import { CircleCheck, X } from 'lucide-svelte'
  import type { OldOrNew } from '../../../types/compareSets'
  import type { LinesDiffResponse } from '../../../types/diff'
  import Tooltip from '../../common/Tooltip.svelte'
  import { T } from '../../../stores/settings/translation.svelte'

  const {
    oldOrNew,
    linesDiffResponse,
  }: { oldOrNew: OldOrNew; linesDiffResponse: LinesDiffResponse | null } = $props()

  const completelyEqual: boolean = $derived(
    linesDiffResponse !== null
      ? linesDiffResponse.diffs.length === 0 ||
          linesDiffResponse.diffs.every((x) => {
            return x.diffKind === 'equal'
          })
      : false
  )

  const charset: string = $derived.by(() => {
    if (linesDiffResponse === null) return ''
    return oldOrNew === 'old' ? linesDiffResponse.oldCharset : linesDiffResponse.newCharset
  })
</script>

<div class="items">
  <div class="left">
    <div>{charset}</div>
  </div>
  <div class="right">
    {#if completelyEqual}
      <Tooltip position="top" messages={T('Completely equal')}>
        <div><CircleCheck /></div>
      </Tooltip>
    {/if}
  </div>
</div>

<style>
  .items {
    flex: 1;
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
</style>
