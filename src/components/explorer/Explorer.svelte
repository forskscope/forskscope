<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { onMount } from 'svelte'
  import type { CompareSet, CompareSetItem, ListDirReponse, OldOrNew } from '../../types'
  import { openDirectoryDialog } from '../../utils/dialog.svelte'
  import Tooltip from '../common/Tooltip.svelte'
  import { T } from '../../stores/settings/translation.svelte'
  import { BINARY_MODE_COMPARE_BUTTON_LABEL, DEFAULT_COMPARE_BUTTON_LABEL } from '../../consts'
  import { pathJoin } from '../../utils/file.svelte'
  import { PATH_SEPARATOR } from '../../stores/file.svelte'
  import { pushCompareSet } from '../../stores/compareSets.svelte'

  interface ExplorePane {
    oldOrNew: OldOrNew
    listDirResponse: ListDirReponse | null
  }

  interface DigestDiff {
    name: string
    equal: boolean
  }

  let oldExplorerPane: ExplorePane = $state({ oldOrNew: 'old', listDirResponse: null })
  let newExplorerPane: ExplorePane = $state({ oldOrNew: 'new', listDirResponse: null })

  let oldSelectedFile: string = $state('')
  let oldBinaryComparisonOnly: boolean = $state(false)
  let newSelectedFile: string = $state('')
  let newBinaryComparisonOnly: boolean = $state(false)

  let dirDigestDiffs: DigestDiff[] = $state([])
  let fileDigestDiffs: DigestDiff[] = $state([])

  const compareButtonLabel: string = $derived(
    oldBinaryComparisonOnly || newBinaryComparisonOnly
      ? BINARY_MODE_COMPARE_BUTTON_LABEL
      : DEFAULT_COMPARE_BUTTON_LABEL
  )

  const compareOnClickEnabled: boolean = $derived(
    0 < oldSelectedFile.length && 0 < newSelectedFile.length
  )

  $effect(() => {
    if (oldExplorerPane.listDirResponse !== null) setDigestDiffs()
  })

  $effect(() => {
    if (newExplorerPane.listDirResponse !== null) setDigestDiffs()
  })

  const lastSlashIndex = (dirpath: string): number => {
    return dirpath.lastIndexOf(PATH_SEPARATOR!)
  }
  const parentDirsPath = (dirpath: string): string => {
    return dirpath.substring(0, lastSlashIndex(dirpath) + 1)
  }
  const dirname = (dirpath: string): string => {
    return dirpath.substring(lastSlashIndex(dirpath) + 1)
  }

  const listDir = async (currentDir: string): Promise<ListDirReponse> => {
    const res = await invoke('list_dir', { currentDir })
      // todo
      .catch((error: unknown) => {
        console.error(error)
        return
      })

    console.log(res) // todo

    return res as ListDirReponse
  }

  onMount(async () => {
    const listDirReponse = await listDir('')

    oldExplorerPane.listDirResponse = listDirReponse
    newExplorerPane.listDirResponse = oldExplorerPane.listDirResponse // todo
  })

  const compareOnClick = () => {
    const oldFilepath: string = pathJoin(
      oldSelectedFile,
      oldExplorerPane.listDirResponse!.currentDir
    )
    const newFilepath: string = pathJoin(
      newSelectedFile,
      newExplorerPane.listDirResponse!.currentDir
    )

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
    pushCompareSet(compareSet)

    // todo reset radio selection
  }

  const selectDir = async (oldOrNew: OldOrNew) => {
    const dirpath = await openDirectoryDialog()
    if (dirpath === null) return
    await changeDir('', dirpath, oldOrNew)
  }

  const changeDir = async (dirname: string, parentDirpath: string, oldOrNew: OldOrNew) => {
    const dirpath = pathJoin(dirname, parentDirpath)
    const listDirResponse = await listDir(dirpath)

    if (oldOrNew === 'old') {
      oldExplorerPane.listDirResponse = listDirResponse
      oldSelectedFile = ''
    } else {
      newExplorerPane.listDirResponse = listDirResponse
      newSelectedFile = ''
    }
  }

  const isRootDir = (dir: string): boolean => {
    if (PATH_SEPARATOR! === '\\') {
      return dir.endsWith(`:${PATH_SEPARATOR!}`)
    } else {
      return dir === PATH_SEPARATOR!
    }
  }

  const syncDir = (oldOrNew: OldOrNew) => {
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

  const filenameOnDblClick = (filename: string) => {
    if (oldExplorerPane.listDirResponse === null || newExplorerPane.listDirResponse === null) return

    const oldFound = oldExplorerPane.listDirResponse.files.find((x) => x.name === filename)
    if (!oldFound) return
    const newFound = newExplorerPane.listDirResponse.files.find((x) => x.name === filename)
    if (!newFound) return

    const oldFilepath: string = pathJoin(filename, oldExplorerPane.listDirResponse!.currentDir)
    const newFilepath: string = pathJoin(filename, newExplorerPane.listDirResponse!.currentDir)

    const compareSet = {
      old: {
        filepath: oldFilepath,
        binaryComparisonOnly: oldFound.binaryComparisonOnly,
      } as CompareSetItem,
      new: {
        filepath: newFilepath,
        binaryComparisonOnly: newFound.binaryComparisonOnly,
      } as CompareSetItem,
    } as CompareSet
    pushCompareSet(compareSet)
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

  const setDigestDiffs = () => {
    if (oldExplorerPane.listDirResponse === null || newExplorerPane.listDirResponse === null) return

    fileDigestDiffs = []
    dirDigestDiffs = []
    if (oldExplorerPane.listDirResponse.currentDir === newExplorerPane.listDirResponse.currentDir)
      return

    let filenames = oldExplorerPane.listDirResponse.files
      .filter((x) => newExplorerPane.listDirResponse!.files.some((y) => y.name === x.name))
      .map((x) => x.name)
    filenames.forEach((x) => {
      invoke('file_digest_diff', {
        filename: x,
        oldDir: oldExplorerPane.listDirResponse!.currentDir,
        newDir: newExplorerPane.listDirResponse!.currentDir,
      }).then((res: unknown) => {
        fileDigestDiffs.push({ name: x, equal: res as boolean } as DigestDiff)
      })
    })

    let dirnames = oldExplorerPane.listDirResponse.dirs.filter((x) =>
      newExplorerPane.listDirResponse!.dirs.some((y) => y === x)
    )
    dirnames.forEach((x) => {
      invoke('dir_digest_diff', {
        dirname: x,
        oldDir: oldExplorerPane.listDirResponse!.currentDir,
        newDir: newExplorerPane.listDirResponse!.currentDir,
      }).then((res: unknown) => {
        dirDigestDiffs.push({ name: x, equal: res as boolean } as DigestDiff)
      })
    })
  }

  const digestDiffClass = (name: string, isDir: boolean): string => {
    let found = isDir
      ? dirDigestDiffs.find((x) => x.name === name)
      : fileDigestDiffs.find((x) => x.name === name)
    if (found) {
      return found.equal ? 'digest-equal' : 'digest-different'
    }
    return ''
  }
</script>

<div class="content explorer-panes">
  {#each [oldExplorerPane, newExplorerPane] as pane}
    {#if pane.listDirResponse !== null}
      <div class={`explorer-pane ${pane.oldOrNew}`}>
        <div class="current-dir">
          <h3 class="path">
            <div class="parent-dirs">{parentDirsPath(pane.listDirResponse.currentDir)}</div>
            <div class="name">{dirname(pane.listDirResponse.currentDir)}</div>
          </h3>
          <div>
            <Tooltip position="top" messages={T('Select dir dialog')}>
              <button class="select-dir" onclick={() => selectDir(pane.oldOrNew)}>⚓️</button>
            </Tooltip>
            <Tooltip position="top" messages={T('Sync dir pos')}>
              <button class="sync-dir" onclick={() => syncDir(pane.oldOrNew)}
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
                ondblclick={() => {
                  changeDir('..', pane.listDirResponse!.currentDir, pane.oldOrNew)
                }}
              >
                ⇡ ..
              </div>
            {/if}

            {#each pane.listDirResponse.dirs as dir}
              <label class="dir"
                ><input
                  type="radio"
                  name={`${pane.oldOrNew}SelectedFile`}
                  checked={dir === (pane.oldOrNew === 'old' ? oldSelectedFile : newSelectedFile)}
                  onchange={(
                    e: Event & {
                      currentTarget: EventTarget & HTMLInputElement
                    }
                  ) => selectedDirOnChange(pane.oldOrNew, e.currentTarget.checked)}
                />
                <div
                  role="button"
                  tabindex="0"
                  class={`name ${digestDiffClass(dir, true)}`}
                  ondblclick={() => {
                    changeDir(dir, pane.listDirResponse!.currentDir, pane.oldOrNew)
                  }}
                >
                  {dir}
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
                  checked={file.name ===
                    (pane.oldOrNew === 'old' ? oldSelectedFile : newSelectedFile)}
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
                <div
                  role="button"
                  tabindex="0"
                  class={`name ${digestDiffClass(file.name, false)}`}
                  ondblclick={() => {
                    filenameOnDblClick(file.name)
                  }}
                >
                  {file.name}
                </div>
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

  .current-dir .path {
    width: calc(100% - 3.2rem);
    margin-left: 0.2rem;
    display: inline-flex;
    overflow: hidden;
    align-items: center;
    font-size: 1rem;
    font-weight: normal;
  }

  /* allow shrinking */
  .current-dir .path .parent-dirs {
    min-width: 0;
    flex-shrink: 1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  /* prevent from truncated */
  .current-dir .path .name {
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
    height: 80vh;
    overflow: auto;
  }

  .dirs-files {
    width: 100%;
    height: fit-content;
  }

  .dirs-files .header,
  .dir,
  .file {
    width: 100%;
    display: flex;
    gap: 1.1rem;
  }

  .dirs-files .header > div,
  .dir > div,
  .file > div {
    flex: 1;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .dirs-files .header > div:nth-of-type(1),
  .dir > .name,
  .file > .name {
    flex: 2;
  }

  .dirs-files .header > div {
    opacity: 0.57;
    font-size: 0.87rem;
    font-weight: bold;
  }

  .parent-dir,
  .dirs-files label {
    margin: 0.08rem 0;
  }

  .parent-dir,
  .dir,
  .file {
    cursor: pointer;
  }

  .dirs-files label input[type='radio'] {
    display: none;
  }

  .dirs-files label:has(input[type='radio']:checked) {
    opacity: 0.93;
  }

  .dirs-files label:not(:has(input[type='radio']:checked)) .name:hover {
    opacity: 0.77;
  }

  .dirs-files label input[type='radio']:checked + .name {
    border-bottom-width: 0.02rem;
    border-bottom-style: solid;
  }

  .dir .name::before,
  .file .name::before {
    width: 2.5em;
    margin-right: 0.4rem;
    display: inline-flex;
    justify-content: flex-end;
    gap: 0.2em;
  }

  .dir .name::before {
    content: '📁';
  }
  .dir .name.digest-different::before {
    content: '⚠📁';
  }
  .dir .name.digest-equal::before {
    content: '✔📁';
  }
  .file .name::before {
    content: '📜';
  }
  .file .name.digest-different::before {
    content: '⚠📜';
  }
  .file .name.digest-equal::before {
    content: '✔📜';
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
