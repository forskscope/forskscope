<script lang="ts">
  import type { LinesDiffResponse } from '../../../../types'
  import DiffFooter from '../templates/DiffFooter.svelte'

  const { linesDiffResponse }: { linesDiffResponse: LinesDiffResponse } = $props()
</script>

<DiffFooter charset={linesDiffResponse.newCharset} />
