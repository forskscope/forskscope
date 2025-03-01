<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { onMount } from 'svelte'
  import type { DiffFilepaths, ListDirReponse, OldOrNew } from '../../types'

  interface ExplorePane {
    oldOrNew: OldOrNew
    listDirResponse: ListDirReponse | null
  }

  const {
    diffFilepathsOnSelected,
  }: { diffFilepathsOnSelected: (diffFilepaths: DiffFilepaths) => void } = $props()

  let oldExplorerPane: ExplorePane = $state({ oldOrNew: 'old', listDirResponse: null })
  let newExplorerPane: ExplorePane = $state({ oldOrNew: 'new', listDirResponse: null })

  let oldSelectedFile: string = $state('')
  let newSelectedFile: string = $state('')

  const diffOnClickEnabled = $derived(0 < oldSelectedFile.length && 0 < newSelectedFile.length)

  onMount(async () => {
    invoke('list_dir', { currentDir: '' })
      .then((res: unknown) => {
        console.log(res) // todo

        oldExplorerPane.listDirResponse = res as ListDirReponse
        newExplorerPane.listDirResponse = oldExplorerPane.listDirResponse // todo
      })
      // todo
      .catch((error: unknown) => {
        console.error(error)
        return
      })
  })

  const diffOnClick = () => {
    const oldFilepath: string = `${oldExplorerPane.listDirResponse!.currentDir}/${oldSelectedFile}`
    const newFilepath: string = `${newExplorerPane.listDirResponse!.currentDir}/${newSelectedFile}`
    diffFilepathsOnSelected({ old: oldFilepath, new: newFilepath } as DiffFilepaths)
  }

  const changeDir = (selectedDir: string, currentDir: string, oldOrNew: OldOrNew) => {
    invoke('list_dir', { currentDir: `${currentDir}/${selectedDir}` })
      .then((res: unknown) => {
        console.log(res) // todo

        const listDirResponse = res as ListDirReponse
        if (oldOrNew === 'old') {
          oldExplorerPane.listDirResponse = listDirResponse
          oldSelectedFile = ''
        } else {
          newExplorerPane.listDirResponse = listDirResponse
          newSelectedFile = ''
        }
      })
      // todo
      .catch((error: unknown) => {
        console.error(error)
        return
      })
  }

  const isRootDir = (dir: string): boolean => {
    return dir === '/' || dir.endsWith(':\\')
  }

  const selectedFileOnChange = (
    e: Event & {
      currentTarget: EventTarget & HTMLInputElement
    },
    oldOrNew: OldOrNew
  ) => {
    const checked = e.currentTarget.checked
    if (!checked) return
    if (oldOrNew === 'old') {
      oldSelectedFile = e.currentTarget.value
    } else {
      newSelectedFile = e.currentTarget.value
    }
  }

  const openWithFileManager = (oldOrNew: OldOrNew) => {
    const dirpath =
      oldOrNew === 'old'
        ? oldExplorerPane.listDirResponse!.currentDir
        : newExplorerPane.listDirResponse!.currentDir
    invoke('open_with_file_manager', { dirpath })
      .then((res: unknown) => {
        console.log(res) // todo
      })
      // todo
      .catch((error: unknown) => {
        console.error(error)
        return
      })
  }
</script>

<h2>Explorer</h2>

<div class="explorer-panes">
  {#each [oldExplorerPane, newExplorerPane] as pane}
    {#if pane.listDirResponse !== null}
      <div class="explorer-pane">
        <div class="current-dir">
          <h3>{pane.listDirResponse.currentDir}</h3>
          <button onclick={() => openWithFileManager(pane.oldOrNew)}>/!/</button>
        </div>
        <div class="dirs-files-wrapper">
          <div class="dirs-files">
            {#if !isRootDir(pane.listDirResponse.currentDir)}
              <button
                class="dir"
                ondblclick={() => changeDir('..', pane.listDirResponse!.currentDir, pane.oldOrNew)}
                >..</button
              >
            {/if}
            {#each pane.listDirResponse.dirs as dir}
              <button
                class="dir"
                ondblclick={() => changeDir(dir, pane.listDirResponse!.currentDir, pane.oldOrNew)}
                >📁 {dir}</button
              >
            {/each}
            {#if 0 < pane.listDirResponse.files.length}
              <div class="file header">
                <div>Name</div>
                <div>Size</div>
                <div>Last Modified</div>
              </div>
            {/if}
            {#each pane.listDirResponse.files as file}
              <label class="file"
                ><input
                  type="radio"
                  name={`${pane.oldOrNew}SelectedFile`}
                  value={file.name}
                  onchange={(e) => selectedFileOnChange(e, pane.oldOrNew)}
                />
                <div>📜 {file.name}</div>
                {#if file.humanReadableSize !== file.bytesSize}
                  <div>{file.humanReadableSize} ({file.bytesSize})</div>
                {:else}
                  <div>{file.bytesSize}</div>
                {/if}
                <div>{file.lastModified}</div>
              </label>
            {/each}
          </div>
        </div>
      </div>
    {/if}
  {/each}
</div>

{#if diffOnClickEnabled}
  <button class="diff" onclick={diffOnClick}>diff</button>
{/if}

<style>
  .explorer-panes,
  .explorer-pane,
  .dirs-files {
    display: flex;
  }
  .explorer-pane,
  .dirs-files {
    flex-direction: column;
  }

  .explorer-panes {
    width: 100%;
    white-space: nowrap;
  }

  .explorer-pane {
    min-width: 0;
    flex: 1;
  }

  .current-dir {
    display: flex;
    justify-content: space-between;
  }

  h3 {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    direction: rtl;
    text-align: right;
  }

  .dirs-files-wrapper {
    height: 60vh;
    overflow: auto;
  }

  .dirs-files {
    width: 100%;
    height: fit-content;
  }

  .dirs-files .dir,
  .dirs-files .file {
    width: 100%;
    display: block;
  }

  .dirs-files .dir {
    width: auto;
    background: inherit;
    color: inherit;
    border: none;
    text-align: left;
    box-shadow: none;
  }

  .dirs-files .file {
    display: flex;
    gap: 1.1rem;
  }

  .dirs-files .file.header > div {
    opacity: 0.57;
    font-size: 0.87rem;
    font-weight: bold;
  }

  .dirs-files .file input[type='radio'] {
    display: none;
  }

  .dirs-files .file:has(input[type='radio']:checked) {
    opacity: 0.87;
  }

  .dirs-files .file input[type='radio']:checked + div {
    border-bottom: 0.02rem solid yellow;
  }

  .dirs-files .file > div {
    flex: 1;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .dirs-files .file > div:nth-of-type(1) {
    flex: 2;
  }

  button.diff {
    width: 90%;
    margin-left: 5%;
  }
</style>
