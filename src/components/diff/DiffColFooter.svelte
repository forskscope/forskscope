<script lang="ts">
  import type { OldOrNew } from '../../types'

  const {
    oldOrNew,
    charset,
  }: {
    oldOrNew: OldOrNew
    charset: string
  } = $props()
</script>

<div>{charset}</div>
