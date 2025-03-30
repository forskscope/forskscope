<script lang="ts">
  import { ArrowUp, Folder } from 'lucide-svelte'
  import ExplorerPaneRow from './template/ExplorerPaneRow.svelte'
  import type { OldOrNew } from '../../../../types/compareSets'
  import {
    changeDir,
    newListDirResponse,
    oldListDirResponse,
    unselectFile,
  } from '../../../../stores/explorer.svelte'

  const MAIN_CLASS: string = 'dir'

  const {
    oldOrNew,
    showsHumanReadableSize,
  }: { oldOrNew: OldOrNew; showsHumanReadableSize: boolean } = $props()

  const listDirResponse = $derived(oldOrNew === 'old' ? $oldListDirResponse : $newListDirResponse)

  const rowOnClick = (i: number) => {
    unselectFile(oldOrNew)
  }

  const rowOnDblClick = (dir: string) => {
    changeDir(oldOrNew, dir)
  }
</script>

<ExplorerPaneRow
  mainClass={MAIN_CLASS}
  {showsHumanReadableSize}
  rowOnClick={() => {
    changeDir(oldOrNew, '..')
  }}
>
  {#snippet fileStatus()}{/snippet}
  {#snippet fileIcon()}<ArrowUp />{/snippet}
  {#snippet fileName()}..{/snippet}
  {#snippet fileHumanReadableSize()}{/snippet}
  {#snippet fileBytesSize()}{/snippet}
  {#snippet fileLastModified()}{/snippet}
</ExplorerPaneRow>
{#if listDirResponse !== null}
  {#each listDirResponse.dirs as dir, i}
    <ExplorerPaneRow
      mainClass={MAIN_CLASS}
      {showsHumanReadableSize}
      rowOnClick={() => {
        rowOnClick(i)
      }}
      rowOnDblClick={() => {
        rowOnDblClick(dir)
      }}
    >
      {#snippet fileStatus()}{/snippet}
      {#snippet fileIcon()}<Folder />{/snippet}
      {#snippet fileName()}{dir}{/snippet}
      {#snippet fileHumanReadableSize()}{/snippet}
      {#snippet fileBytesSize()}{/snippet}
      {#snippet fileLastModified()}{/snippet}
    </ExplorerPaneRow>
  {/each}
{/if}
