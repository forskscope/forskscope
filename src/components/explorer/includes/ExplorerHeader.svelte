<script lang="ts">
  import { ArrowLeft, ArrowRight, ArrowUp, Folder } from 'lucide-svelte'
  import Tooltip from '../../common/Tooltip.svelte'
  import { T } from '../../../stores/settings/translation.svelte'
  import type { OldOrNew } from '../../../types/compareSets'
  import { extractDirname, parentDirsPath } from '../helpers.svelte'
  import { openDirectoryDialog } from '../../../utils/dialog.svelte'
  import {
    changeDir,
    newListDirResponse,
    oldListDirResponse,
  } from '../../../stores/explorer.svelte'

  const {
    oldOrNew,
  }: {
    oldOrNew: OldOrNew
  } = $props()

  const listDirResponse = $derived(oldOrNew === 'old' ? $oldListDirResponse : $newListDirResponse)

  const selectDir = async (oldOrNew: OldOrNew) => {
    const dirpath = await openDirectoryDialog()
    if (dirpath === null) return
    await changeDir(oldOrNew, '', dirpath)
  }
</script>

<div class="current-dir">
  <h3 class="path">
    {#if listDirResponse !== null}
      <div class="parent-dirs">{parentDirsPath(listDirResponse.currentDir)}</div>
      <div class="name">{extractDirname(listDirResponse.currentDir)}</div>
    {/if}
  </h3>
  <div class="buttons">
    <Tooltip position="top" messages={T('Select dir dialog')}>
      <button class="select-dir" onclick={() => selectDir(oldOrNew)}>
        <Folder />
      </button>
    </Tooltip>
    <Tooltip position="top" messages={T('Go back')}>
      <!-- todo: history back -->
      <button class="go-back" onclick={() => changeDir(oldOrNew, '.')} disabled>
        <ArrowLeft />
      </button>
    </Tooltip>
    <Tooltip position="top" messages={T('Go forward')}>
      <!-- todo: history proceed -->
      <button class="go-forward" onclick={() => changeDir(oldOrNew, '.')} disabled>
        <ArrowRight />
      </button>
    </Tooltip>
    <Tooltip position="top" messages={T('Move up')}>
      <button class="move-up" onclick={() => changeDir(oldOrNew, '..')}>
        <ArrowUp />
      </button>
    </Tooltip>
  </div>
</div>

<style>
  .current-dir {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.2rem;
  }

  .path {
    display: inline-flex;
    align-items: center;
    gap: 0.02rem;
    font-size: 1rem;
    font-weight: normal;
    overflow: hidden;
  }

  /* allow shrinking */
  .parent-dirs {
    min-width: 0;
    flex-shrink: 1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  /* prevent from truncated */
  .path .name {
    flex-shrink: 0;
  }

  .buttons {
    flex: 1 0 1;
  }

  .buttons button {
    padding: 0.2rem 0.4rem;
  }

  .buttons .select-dir {
    margin-right: 0.3rem;
    padding: 0.3rem 0.6rem;
  }
</style>
