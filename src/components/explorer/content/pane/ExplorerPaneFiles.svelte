<script lang="ts">
  import { Check, File, TriangleAlert } from 'lucide-svelte'
  import type { FileAttr } from '../../../../types/file'
  import ExplorerPaneRow from './template/ExplorerPaneRow.svelte'
  import type { OldOrNew } from '../../../../types/compareSets'
  import {
    newListDirResponse,
    oldListDirResponse,
    isSelected,
    selectFile,
    oldSelectedFileIndex,
    newSelectedFileIndex,
    fileDigestDiffs,
    statusIconName,
    pushCompareSetIfReady,
  } from '../../../../stores/explorer.svelte'

  const MAIN_CLASS_BASE: string = 'file'
  const MAIN_CLASS_BINARY_COMPARISON_ONLY: string = 'binary-comparison-only'
  const MAIN_CLASS_SELECTED: string = 'selected'

  const {
    oldOrNew,
    showsHumanReadableSize,
  }: { oldOrNew: OldOrNew; showsHumanReadableSize: boolean } = $props()

  const listDirResponse = $derived(oldOrNew === 'old' ? $oldListDirResponse : $newListDirResponse)

  const _selectedFileIndex = $derived(
    oldOrNew === 'old' ? $oldSelectedFileIndex : $newSelectedFileIndex
  )

  const mainClass = (file: FileAttr, index: number): string => {
    let ret: string[] = [MAIN_CLASS_BASE]
    if (file.binaryComparisonOnly) ret.push(MAIN_CLASS_BINARY_COMPARISON_ONLY)
    if (isSelected(oldOrNew, index)) ret.push(MAIN_CLASS_SELECTED)
    return ret.join(' ')
  }

  const rowOnClick = (i: number) => {
    selectFile(oldOrNew, i)
  }

  const rowOnDblClick = (i: number) => {
    pushCompareSetIfReady(oldOrNew, i)
  }
</script>

{#if listDirResponse !== null}
  {#key [_selectedFileIndex, $fileDigestDiffs]}
    {#each listDirResponse.files as file, i}
      <ExplorerPaneRow
        mainClass={mainClass(file, i)}
        {showsHumanReadableSize}
        rowOnClick={() => {
          rowOnClick(i)
        }}
        rowOnDblClick={() => {
          rowOnDblClick(i)
        }}
      >
        {#snippet fileStatus()}
          <!-- todo: dynamic icon rendering -->
          {@const Icon = statusIconName(file.name, false)}
          {#if Icon === 'TriangleAlert'}
            <TriangleAlert />
          {:else if Icon === 'Check'}
            <Check />
          {/if}
        {/snippet}
        {#snippet fileIcon()}<File />{/snippet}
        {#snippet fileName()}{file.name}{/snippet}
        {#snippet fileHumanReadableSize()}{file.humanReadableSize}{/snippet}
        {#snippet fileBytesSize()}{file.bytesSize}{/snippet}
        {#snippet fileLastModified()}{file.lastModified}{/snippet}
      </ExplorerPaneRow>
    {/each}
  {/key}
{:else}
  <!-- todo: loading -->
  (...... Loading ......)
{/if}
