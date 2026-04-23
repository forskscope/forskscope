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
  const ROW_HEIGHT = 28 // Approximate height of a row in pixels
  const BUFFER_SIZE = 10 // Number of extra items to render above and below

  let container: HTMLElement | undefined = $state()
  let scrollTop = $state(0)
  let containerHeight = $state(0)

  const visibleRange = $derived.by(() => {
    if (!listDirResponse) return { start: 0, end: 0 }

    const totalItems = listDirResponse.files.length
    const start = Math.floor(scrollTop / ROW_HEIGHT)
    const visibleCount = Math.ceil(containerHeight / ROW_HEIGHT)

    const startIndex = Math.max(0, start - BUFFER_SIZE)
    const endIndex = Math.min(totalItems, start + visibleCount + BUFFER_SIZE)

    return { start: startIndex, end: endIndex }
  })

  const onScroll = (e: Event) => {
    const target = e.target as HTMLElement
    scrollTop = target.scrollTop
  }

  $effect(() => {
    if (container) {
      const resizeObserver = new ResizeObserver((entries) => {
        for (const entry of entries) {
          containerHeight = entry.contentRect.height
        }
      })
      resizeObserver.observe(container)
      return () => resizeObserver.disconnect()
    }
  })
</script>

{#if listDirResponse !== null}
  <div
    class="virtual-scroll-container"
    bind:this={container}
    onscroll={onScroll}
    style="height: 100%; overflow-y: auto; position: relative;"
  >
    <div style="height: {listDirResponse.files.length * ROW_HEIGHT}px; position: relative;">
      <div style="position: absolute; top: {visibleRange.start * ROW_HEIGHT}px; left: 0; right: 0;">
        {#key [_selectedFileIndex, $fileDigestDiffs]}
          {#each listDirResponse.files.slice(visibleRange.start, visibleRange.end) as file, i}
            {@const actualIndex = visibleRange.start + i}
            <ExplorerPaneRow
              mainClass={mainClass(file, actualIndex)}
              {showsHumanReadableSize}
              rowOnClick={() => {
                rowOnClick(actualIndex)
              }}
              rowOnDblClick={() => {
                rowOnDblClick(actualIndex)
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
      </div>
    </div>
  </div>
{:else}
  <!-- todo: loading -->
  (...... Loading ......)
{/if}
