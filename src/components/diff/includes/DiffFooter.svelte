<script lang="ts">
  import type { OldOrNew } from '../../../types/compareSets.svelte'
  import type { LinesDiffResponse } from '../../../types/diff.svelte'

  const {
    oldOrNew,
    linesDiffResponse,
  }: { oldOrNew: OldOrNew; linesDiffResponse: LinesDiffResponse | null } = $props()

  const charset: string = $derived.by(() => {
    if (linesDiffResponse === null) return ''
    return oldOrNew === 'old' ? linesDiffResponse.oldCharset : linesDiffResponse.newCharset
  })
</script>

<div>{charset}</div>
