<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { onMount } from 'svelte'
  import type { CompareSet, CompareSetItem, ListDirReponse, OldOrNew } from '../../types'
  import { openDirectoryDialog } from '../../scripts'
  import Tooltip from '../common/Tooltip.svelte'
  import { T } from '../../stores/translation.svelte'
  import { BINARY_MODE_COMPARE_BUTTON_LABEL, DEFAULT_COMPARE_BUTTON_LABEL } from '../../consts'

  interface ExplorePane {
    oldOrNew: OldOrNew
    listDirResponse: ListDirReponse | null
  }

  interface CompareSetItemSource {
    filename: string
    binaryComparisonOnly: boolean
  }

  const { compareSetOnSelected }: { compareSetOnSelected: (compareSet: CompareSet) => void } =
    $props()

  let oldExplorerPane: ExplorePane = $state({ oldOrNew: 'old', listDirResponse: null })
  let newExplorerPane: ExplorePane = $state({ oldOrNew: 'new', listDirResponse: null })

  let oldSelectedFile: string = $state('')
  let oldBinaryComparisonOnly: boolean = $state(false)
  let newSelectedFile: string = $state('')
  let newBinaryComparisonOnly: boolean = $state(false)

  const compareButtonLabel: string = $derived(
    oldBinaryComparisonOnly || newBinaryComparisonOnly
      ? BINARY_MODE_COMPARE_BUTTON_LABEL
      : DEFAULT_COMPARE_BUTTON_LABEL
  )

  const compareOnClickEnabled: boolean = $derived(
    0 < oldSelectedFile.length && 0 < newSelectedFile.length
  )

  const lastSlashIndex = (dirpath: string): number => {
    return dirpath.lastIndexOf('/')
  }
  const parentDirsPath = (dirpath: string): string => {
    return dirpath.substring(0, lastSlashIndex(dirpath) + 1)
  }
  const dirname = (dirpath: string): string => {
    return dirpath.substring(lastSlashIndex(dirpath) + 1)
  }

  onMount(async () => {
    const res = await invoke('list_dir', { currentDir: '' })
      // todo
      .catch((error: unknown) => {
        console.error(error)
        return
      })

    console.log(res) // todo

    oldExplorerPane.listDirResponse = res as ListDirReponse
    newExplorerPane.listDirResponse = oldExplorerPane.listDirResponse // todo
  })

  const compareOnClick = () => {
    const oldFilepath: string = `${oldExplorerPane.listDirResponse!.currentDir}/${oldSelectedFile}`
    const newFilepath: string = `${newExplorerPane.listDirResponse!.currentDir}/${newSelectedFile}`

    const compareSet = {
      old: {
        filepath: oldFilepath,
        binaryComparisonOnly: oldBinaryComparisonOnly,
      } as CompareSetItem,
      new: {
        filepath: newFilepath,
        binaryComparisonOnly: newBinaryComparisonOnly,
      } as CompareSetItem,
    } as CompareSet
    compareSetOnSelected(compareSet)

    // todo reset radio selection
  }

  const selectDir = async (oldOrNew: OldOrNew) => {
    const dirpath = await openDirectoryDialog()
    if (dirpath === null) return
    await changeDir(dirpath, oldOrNew)
  }

  const changeDir = async (dirpath: string, oldOrNew: OldOrNew) => {
    const res: unknown = await invoke('list_dir', { currentDir: dirpath })
      // todo
      .catch((error: unknown) => {
        console.error(error)
        return
      })

    console.log(res) // todo

    const listDirResponse = res as ListDirReponse
    if (oldOrNew === 'old') {
      oldExplorerPane.listDirResponse = listDirResponse
      oldSelectedFile = ''
    } else {
      newExplorerPane.listDirResponse = listDirResponse
      newSelectedFile = ''
    }
  }

  const isRootDir = (dir: string): boolean => {
    return dir === '/' || dir.endsWith(':\\')
  }

  const copyCurrentDir = (oldOrNew: OldOrNew) => {
    if (oldOrNew === 'old') {
      newExplorerPane.listDirResponse = oldExplorerPane.listDirResponse
    } else {
      oldExplorerPane.listDirResponse = newExplorerPane.listDirResponse
    }
  }

  const selectedDirOnChange = (oldOrNew: OldOrNew, checked: boolean) => {
    if (!checked) return

    if (oldOrNew === 'old') {
      oldSelectedFile = ''
      oldBinaryComparisonOnly = false
    } else {
      newSelectedFile = ''
      newBinaryComparisonOnly = false
    }
  }

  const selectedFileOnChange = (
    oldOrNew: OldOrNew,
    checked: boolean,
    filename: string,
    binaryComparisonOnly: boolean
  ) => {
    if (!checked) return

    if (oldOrNew === 'old') {
      oldSelectedFile = filename
      oldBinaryComparisonOnly = binaryComparisonOnly
    } else {
      newSelectedFile = filename
      newBinaryComparisonOnly = binaryComparisonOnly
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

<div class="content explorer-panes">
  {#each [oldExplorerPane, newExplorerPane] as pane}
    {#if pane.listDirResponse !== null}
      <div class={`explorer-pane ${pane.oldOrNew}`}>
        <div class="current-dir">
          <h3 class="dirpath">
            <div class="parent-dirs">{parentDirsPath(pane.listDirResponse.currentDir)}</div>
            <div class="dirname">{dirname(pane.listDirResponse.currentDir)}</div>
          </h3>
          <div>
            <Tooltip position="top" messages={T('Select dir dialog')}>
              <button class="select-dir" onclick={() => selectDir(pane.oldOrNew)}>⚓️</button>
            </Tooltip>
            <Tooltip position="top" messages={T('Sync dir pos')}>
              <button class="sync-dir" onclick={() => copyCurrentDir(pane.oldOrNew)}
                >{#if pane.oldOrNew === 'old'}→{:else}←{/if}</button
              >
            </Tooltip>
            <Tooltip position="top" messages={T('Run file manager')}>
              <button class="file-manager" onclick={() => openWithFileManager(pane.oldOrNew)}
                >📦️</button
              >
            </Tooltip>
          </div>
        </div>
        <div class="dirs-files-wrapper">
          <div class="dirs-files">
            {#if 0 < pane.listDirResponse.files.length}
              <div class="header">
                <div>{T('Name')}</div>
                <div>{T('Size')}</div>
                <div>{T('Last modified')}</div>
              </div>
            {/if}
            {#if !isRootDir(pane.listDirResponse.currentDir)}
              <div
                role="button"
                tabindex="0"
                class="parent-dir"
                ondblclick={() =>
                  changeDir(`${pane.listDirResponse!.currentDir}/..`, pane.oldOrNew)}
              >
                ⇡ ..
              </div>
            {/if}

            {#each pane.listDirResponse.dirs as dir}
              <label class="dir"
                ><input
                  type="radio"
                  name={`${pane.oldOrNew}SelectedFile`}
                  onchange={(
                    e: Event & {
                      currentTarget: EventTarget & HTMLInputElement
                    }
                  ) => selectedDirOnChange(pane.oldOrNew, e.currentTarget.checked)}
                />
                <div
                  role="button"
                  tabindex="0"
                  ondblclick={() =>
                    changeDir(`${pane.listDirResponse!.currentDir}/${dir}`, pane.oldOrNew)}
                >
                  📁 {dir}
                </div>
                <div></div>
                <div></div>
              </label>
            {/each}

            {#each pane.listDirResponse.files as file}
              <label class={`file ${file.binaryComparisonOnly ? 'binary-comparison-only' : ''}`}
                ><input
                  type="radio"
                  name={`${pane.oldOrNew}SelectedFile`}
                  onchange={(
                    e: Event & {
                      currentTarget: EventTarget & HTMLInputElement
                    }
                  ) =>
                    selectedFileOnChange(
                      pane.oldOrNew,
                      e.currentTarget.checked,
                      file.name,
                      file.binaryComparisonOnly
                    )}
                />
                <div role="button" tabindex="0">📜 {file.name}</div>
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

<div class="footer">
  <h2>Explorer</h2>
  {#if compareOnClickEnabled}
    <button class="compare" onclick={compareOnClick}>{T(compareButtonLabel)}</button>
  {:else}
    <span>{T('Select files to compare')}</span>
  {/if}
</div>

<style>
  .content {
    height: calc(100vh - 3.3rem);
  }

  .footer {
    position: relative;
    height: 1.7rem;
    text-align: center;
  }

  h2 {
    position: absolute;
    right: 0.4rem;
    top: 0.3rem;
    font-size: 0.8rem;
    pointer-events: none;
  }

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
    align-items: center;
  }

  .dirpath {
    width: calc(100% - 3.2rem);
    margin-left: 0.2rem;
    display: inline-flex;
    overflow: hidden;
    align-items: center;
    font-size: 1rem;
    font-weight: normal;
  }

  /* Allows shrinking */
  .parent-dirs {
    min-width: 0;
    flex-shrink: 1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  /* Prevent from truncated */
  .dirname {
    flex-shrink: 0;
    margin-left: 0.02rem;
  }

  .current-dir button {
    padding: 0.2rem 0.4rem;
  }

  .current-dir button.select-dir {
    margin-right: 0.3rem;
    padding: 0.3rem 0.6rem;
  }

  .dirs-files-wrapper {
    height: 60vh;
    overflow: auto;
  }

  .dirs-files {
    width: 100%;
    height: fit-content;
  }

  .dirs-files .header,
  .dirs-files label {
    width: 100%;
    display: flex;
    gap: 1.1rem;
  }

  .dirs-files .header > div,
  .dirs-files label > div {
    flex: 1;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .dirs-files .header > div:nth-of-type(1),
  .dirs-files label > div:nth-of-type(1) {
    flex: 2;
  }

  .dirs-files .header > div {
    opacity: 0.57;
    font-size: 0.87rem;
    font-weight: bold;
  }

  .dirs-files .parent-dir,
  .dirs-files label {
    margin: 0.08rem 0;
  }

  .dirs-files .parent-dir,
  .dirs-files label div:first-of-type {
    cursor: pointer;
  }

  .dirs-files label input[type='radio'] {
    display: none;
  }

  .dirs-files label:has(input[type='radio']:checked) {
    opacity: 0.87;
  }

  .dirs-files label input[type='radio']:checked + div {
    border-bottom-width: 0.02rem;
    border-bottom-style: solid;
  }

  .file.binary-comparison-only {
    opacity: 0.5;
  }

  .footer button.compare {
    width: 12rem;
    padding: 0.2rem 0;
  }

  .footer span {
    opacity: 0.57;
  }
</style>
