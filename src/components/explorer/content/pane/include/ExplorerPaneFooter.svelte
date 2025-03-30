<script lang="ts">
  import { numberOfDirs, numberOfFiles } from '../../../../../stores/explorer.svelte'
  import { T } from '../../../../../stores/settings/translation.svelte'
  import type { OldOrNew } from '../../../../../types/compareSets'

  const { oldOrNew }: { oldOrNew: OldOrNew } = $props()

  const numberOfDirsText: string = $derived.by(() => {
    const num = numberOfDirs(oldOrNew)
    if (num === null) return ''
    return `${num} ${T('dir(s)')}`
  })

  const numberOfFilesText: string = $derived.by(() => {
    const num = numberOfFiles(oldOrNew)
    if (num === null) return ''
    return `${num} ${T('file(s)')}`
  })
</script>

<div class="row">
  <div class="col">{numberOfDirsText}</div>
  <div class="col">{numberOfFilesText}</div>
</div>
