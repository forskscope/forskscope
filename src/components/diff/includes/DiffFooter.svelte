<script lang="ts">
  import type { LinesDiffResponse, OldOrNew } from '../../../types'

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
