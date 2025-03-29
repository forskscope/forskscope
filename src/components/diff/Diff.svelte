<script lang="ts">
  import { onMount } from 'svelte'
  import { invokeWithGuard } from '../../utils/backend.svelte'
  import type { CompareSet, OldOrNew } from '../../types/compareSets.svelte'
  import type {
    CharsDiffResponse,
    LinesDiffResponse,
    MergeHistoryItem,
  } from '../../types/diff.svelte'
  import type { BackendCommandResult } from '../../types/backend.svelte'
  import {
    getCompareSet,
    isActiveCompareSetIndex,
    removeCompareSet,
    updateCompareSet,
  } from '../../stores/compareSets.svelte'
  import View from '../../layouts/default/view/View.svelte'
  import DiffHeaderDivider from './includes/DiffHeaderDivider.svelte'
  import DiffContent from './content/DiffContent.svelte'
  import DiffFooterDivider from './includes/DiffFooterDivider.svelte'
  import DiffHeader from './includes/DiffHeader.svelte'
  import DiffFooter from './includes/DiffFooter.svelte'
  import DiffContentDivider from './content/DiffContentDivider.svelte'
  import { DIFF_LINE_HEIGHT, LINES_DIFF_CLASS_PREFIX } from './consts'

  const { compareSetIndex }: { compareSetIndex: number } = $props()

  let compareSet: CompareSet = $state(getCompareSet(compareSetIndex))

  let linesDiffResponse: LinesDiffResponse | null = $state(null)
  let charsDiffResponse: CharsDiffResponse | null = $state(null)

  // todo
  let showsCharsDiffs: boolean = $state(false)
  let focusedLinesDiffIndex: number | null = $state(null)

  const oldFilepath: string = $derived(compareSet.old.filepath)
  const newFilepath: string = $derived(compareSet.new.filepath)

  const firstFocusedLinesDiffIndex: number = $derived.by(() => {
    if (!linesDiffResponse) return -1
    return linesDiffResponse.diffs.findIndex((x) => x.diffKind !== 'equal')
  })

  const lastFocusedLinesDiffIndex: number = $derived.by(() => {
    if (!linesDiffResponse) return -1
    return linesDiffResponse.diffs.findLastIndex((x) => x.diffKind !== 'equal')
  })

  const visible: boolean = $derived(isActiveCompareSetIndex(compareSetIndex))

  const mergeHistory: MergeHistoryItem[] = []

  onMount(async () => {
    await diffLines()
  })

  const diffLines = async () => {
    const res: BackendCommandResult = await invokeWithGuard('diff_filepaths', {
      old: oldFilepath,
      new: newFilepath,
    })
    if (res.isError) {
      removeCompareSet(compareSetIndex)
      return
    }

    reset()
    linesDiffResponse = res.response as LinesDiffResponse
  }

  const diffChars = async () => {
    if (charsDiffResponse !== null) return

    const replaceLinesDiffs = linesDiffResponse!.diffs.filter((x) => x.diffKind === 'replace')
    if (replaceLinesDiffs.length === 0) return

    const res: BackendCommandResult = await invokeWithGuard('diff_chars', {
      linesDiffs: replaceLinesDiffs,
    })

    charsDiffResponse = res.response as CharsDiffResponse
  }

  const filepathOnChange = async (oldOrNew: OldOrNew, filepath: string) => {
    if (oldOrNew === 'old') {
      const _oldFilepath = filepath
      compareSet = await updateCompareSet(compareSetIndex, _oldFilepath, compareSet.new.filepath)
    } else {
      const _newFilepath = filepath
      compareSet = await updateCompareSet(compareSetIndex, compareSet.old.filepath, _newFilepath)
    }

    await diffLines()
  }

  const reset = () => {
    linesDiffResponse = null
    charsDiffResponse = null

    focusedLinesDiffIndex = null
    showsCharsDiffs = false
  }

  const toggleCharsDiffs = () => {
    diffChars()
    showsCharsDiffs = !showsCharsDiffs
  }

  const focusedLinesDiffIndexOnChange = (delta: number) => {
    if (!linesDiffResponse) return

    if (focusedLinesDiffIndex === null) {
      focusedLinesDiffIndex = firstFocusedLinesDiffIndex
      return
    }

    if (focusedLinesDiffIndex === firstFocusedLinesDiffIndex && delta < 0) return
    if (focusedLinesDiffIndex === lastFocusedLinesDiffIndex && 0 < delta) return

    if (0 < delta) {
      focusedLinesDiffIndex = linesDiffResponse.diffs.findIndex((x, i) => {
        return x.diffKind !== 'equal' && focusedLinesDiffIndex! < i
      })
    } else {
      focusedLinesDiffIndex = linesDiffResponse.diffs.findLastIndex((x, i) => {
        return x.diffKind !== 'equal' && i < focusedLinesDiffIndex!
      })
    }

    // scroll into view
    document
      .querySelector(`.diff .content .new .focused`)!
      .scrollIntoView({ behavior: 'smooth', block: 'center', inline: 'start' })
  }

  const mergeOnClick = (index: number) => {
    if (!linesDiffResponse) return

    const merged = linesDiffResponse.diffs[index]

    mergeHistory.push({
      diffIndex: merged.diffIndex,
      orgNewLines: merged.newLines,
      orgDiffKind: merged.diffKind,
    } as MergeHistoryItem)

    merged.newLines = linesDiffResponse.diffs[index].oldLines
    merged.diffKind = 'equal'
  }

  const undoMergeOnClick = () => {
    const mergeHistoryItem = mergeHistory.pop()
    if (!linesDiffResponse || !mergeHistoryItem) return
    const reverted = linesDiffResponse.diffs[mergeHistoryItem.diffIndex]
    reverted.newLines = mergeHistoryItem.orgNewLines
    reverted.diffKind = mergeHistoryItem.orgDiffKind
  }
</script>

{#if linesDiffResponse !== null}
  <View
    mainClass="diff"
    customStyle={`--line-height: ${DIFF_LINE_HEIGHT};`}
    {visible}
    scrollSyncs={true}
  >
    {#snippet leftHeader()}
      <DiffHeader oldOrNew="old" {compareSet} {filepathOnChange} />
    {/snippet}
    {#snippet headerDivider()}
      <DiffHeaderDivider {focusedLinesDiffIndexOnChange} />
    {/snippet}
    {#snippet rightHeader()}
      <DiffHeader oldOrNew="new" {compareSet} {filepathOnChange} />
    {/snippet}

    {#snippet leftContent()}
      <DiffContent
        oldOrNew="old"
        {linesDiffResponse}
        {charsDiffResponse}
        {showsCharsDiffs}
        {focusedLinesDiffIndex}
      />
    {/snippet}
    {#snippet contentDivider()}
      <DiffContentDivider {linesDiffResponse} {focusedLinesDiffIndex} {mergeOnClick} />
    {/snippet}
    {#snippet rightContent()}
      <DiffContent
        oldOrNew="new"
        {linesDiffResponse}
        {charsDiffResponse}
        {showsCharsDiffs}
        {focusedLinesDiffIndex}
      />
    {/snippet}

    {#snippet leftFooter()}
      <DiffFooter oldOrNew="old" {linesDiffResponse} />
    {/snippet}
    {#snippet footerDivider()}
      <DiffFooterDivider {toggleCharsDiffs} {undoMergeOnClick} />
    {/snippet}
    {#snippet rightFooter()}
      <DiffFooter oldOrNew="new" {linesDiffResponse} />
    {/snippet}
  </View>
{:else}
  <!-- todo: loading -->
  (...... Loading ......)
{/if}
