<script lang="ts">
  import type { OldOrNew } from '../../../../../types/compareSets'

  const { oldOrNew }: { oldOrNew: OldOrNew } = $props()

  let numberOfDirsText: string = $state('')
  let numberOfFilesText: string = $state('')

  // todo: show detail
  // const _listDirResponse: ListDirResponse | null = listDirResponse(oldOrNew)
  // if (listDirResponse(oldOrNew) === null) return
  // numberOfDirsText = `${_listDirResponse!.dirs.length} ${T('dir(s)')}`
  // numberOfFilesText = `${_listDirResponse!.files.length} ${T('file(s)')}`
</script>

<div class="row">
  <div class="col">
    {numberOfDirsText}
  </div>
  <div class="col">
    {numberOfFilesText}
  </div>
</div>
