<script lang="ts">
  import {
    newListDirResponse,
    oldListDirResponse,
    setDigestDiffs,
  } from '../../../stores/explorer.svelte'
  import type { OldOrNew } from '../../../types/compareSets'
  import type { ListDirResponse } from '../../../types/file'
  import { listDir } from '../helpers.svelte'
  import ExplorerPaneDirs from './pane/ExplorerPaneDirs.svelte'
  import ExplorerPaneFiles from './pane/ExplorerPaneFiles.svelte'
  import ExplorerPaneFooter from './pane/include/ExplorerPaneFooter.svelte'
  import ExplorerPaneHeader from './pane/include/ExplorerPaneHeaders.svelte'
  import './style.scss'

  const {
    oldOrNew,
    showsHumanReadableSize,
  }: {
    oldOrNew: OldOrNew
    showsHumanReadableSize: boolean
  } = $props()

  const listDirResponse: ListDirResponse | null = $derived(
    oldOrNew === 'old' ? $oldListDirResponse : $newListDirResponse
  )

  $effect(() => {
    if (listDirResponse !== null) setDigestDiffs()
  })
</script>

<div class={`explorer-container ${oldOrNew}`}>
  <div class="header">
    <div class="content">
      <div class="headers">
        <ExplorerPaneHeader {showsHumanReadableSize} />
      </div>
    </div>
  </div>

  <div class="body">
    <div class="content">
      <div class="dirs">
        <ExplorerPaneDirs {oldOrNew} {showsHumanReadableSize} />
      </div>
      <div class="files">
        <ExplorerPaneFiles {oldOrNew} {showsHumanReadableSize} />
      </div>
    </div>
  </div>

  <div class="footer">
    <div class="content">
      <ExplorerPaneFooter {oldOrNew} />
    </div>
  </div>
</div>
