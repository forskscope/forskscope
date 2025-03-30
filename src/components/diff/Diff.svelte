<script lang="ts">
  import { onMount } from 'svelte'
  import { invokeWithGuard } from '../../utils/backend.svelte'
  import type { CompareSet, OldOrNew } from '../../types/compareSets'
  import type { CharsDiffResponse, LinesDiffResponse, MergeHistoryItem } from '../../types/diff'
  import type { BackendCommandResult } from '../../types/backend'
  import {
    getCompareSet,
    isActiveCompareSet,
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
  import { DIFF_LINE_HEIGHT } from './consts'
  import {
    deltaModifyingFocusedDiffLinesIndex,
    diffCharsFromLinesDiffResponse,
    scrollIntoFocusedDiffLines,
    switchCharsDiffLinesList,
    switchLinesDiffs,
  } from './helpers.svelte'

  const { compareSetIndex }: { compareSetIndex: number } = $props()

  let compareSet: CompareSet = $state(getCompareSet(compareSetIndex))

  let linesDiffResponse: LinesDiffResponse | null = $state(null)
  let charsDiffResponse: CharsDiffResponse | null = $state(null)

  // todo
  let showsCharsDiffs: boolean = $state(false)
  let focusedLinesDiffIndex: number | null = $state(null)

  const oldFilepath: string = $derived(compareSet.old.filepath)
  const newFilepath: string = $derived(compareSet.new.filepath)

  const visible: boolean = $derived(isActiveCompareSet(compareSetIndex))

  const mergeHistory: MergeHistoryItem[] = $state([])

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

    const res = await diffCharsFromLinesDiffResponse(linesDiffResponse!)
    if (res === null) return
    charsDiffResponse = res
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

  const focusedLinesDiffIndexOnChange = (delta: number) => {
    focusedLinesDiffIndex = deltaModifyingFocusedDiffLinesIndex(
      delta,
      focusedLinesDiffIndex,
      linesDiffResponse
    )
    scrollIntoFocusedDiffLines()
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

  const toggleCharsDiffs = () => {
    diffChars()
    showsCharsDiffs = !showsCharsDiffs
  }

  const switchFilepaths = async () => {
    compareSet = await updateCompareSet(
      compareSetIndex,
      compareSet.new.filepath,
      compareSet.old.filepath
    )

    if (linesDiffResponse === null) {
      diffLines()
      return
    }

    linesDiffResponse.diffs = switchLinesDiffs(linesDiffResponse.diffs)

    const oldCharset = linesDiffResponse.oldCharset
    linesDiffResponse.oldCharset = linesDiffResponse.newCharset
    linesDiffResponse.newCharset = oldCharset

    if (charsDiffResponse !== null) {
      charsDiffResponse.diffs = switchCharsDiffLinesList(charsDiffResponse.diffs)
    }
  }

  const undoMergeHistory = () => {
    const mergeHistoryItem = mergeHistory.pop()
    if (!linesDiffResponse || !mergeHistoryItem) return
    const reverted = linesDiffResponse.diffs[mergeHistoryItem.diffIndex]
    reverted.newLines = mergeHistoryItem.orgNewLines
    reverted.diffKind = mergeHistoryItem.orgDiffKind
  }

  // todo: save as
  // const saveAsOnClick = async () => {
  //   const filepath = await saveFileDialog(newFilepath!).catch((error: unknown) => {
  //     console.error(error)
  //     return
  //   })
  //   if (!filepath) return
  //   await invoke('save', {
  //     filepath: filepath,
  //     content: linesDiffs.reduce((a, b) => `${a}${b.newLines.join('')}`, ''),
  //     charset: newCharset,
  //   }).catch((error: unknown) => {
  //     console.error(error)
  //     return
  //   })
  // }

  // todo: keyboard shotcuts
  // const onKeyDown = (
  //   e: KeyboardEvent & {
  //     currentTarget: EventTarget & HTMLDivElement
  //   }
  // ) => {
  //   switch (e.key) {
  //     case 'w': {
  //       if (e.ctrlKey) {
  //         removeActiveCompareSet()
  //       }
  //     }
  //     case 'F7': {
  //       focusedLinesDiffIndex = prevLinesDiffIndex
  //       break
  //     }
  //     case 'F8': {
  //       focusedLinesDiffIndex = nextLinesDiffIndex
  //       break
  //     }
  //     default:
  //   }
  // }
</script>

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
    {#if linesDiffResponse !== null}
      <DiffContent
        oldOrNew="old"
        {linesDiffResponse}
        {charsDiffResponse}
        {showsCharsDiffs}
        {focusedLinesDiffIndex}
      />
    {:else}
      <!-- todo: loading -->
      (...... Loading ......)
    {/if}
  {/snippet}
  {#snippet contentDivider()}
    {#if linesDiffResponse !== null}
      <DiffContentDivider {linesDiffResponse} {focusedLinesDiffIndex} {mergeOnClick} />
    {/if}
  {/snippet}
  {#snippet rightContent()}
    {#if linesDiffResponse !== null}
      <DiffContent
        oldOrNew="new"
        {linesDiffResponse}
        {charsDiffResponse}
        {showsCharsDiffs}
        {focusedLinesDiffIndex}
      />
    {:else}
      <!-- todo: loading -->
      (...... Loading ......)
    {/if}
  {/snippet}

  {#snippet leftFooter()}
    <DiffFooter oldOrNew="old" {linesDiffResponse} />
  {/snippet}
  {#snippet footerDivider()}
    <DiffFooterDivider {mergeHistory} {toggleCharsDiffs} {switchFilepaths} {undoMergeHistory} />
  {/snippet}
  {#snippet rightFooter()}
    <DiffFooter oldOrNew="new" {linesDiffResponse} />
  {/snippet}
</View>
