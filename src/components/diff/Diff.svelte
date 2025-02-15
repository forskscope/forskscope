<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import FileHandle from './FileHandle.svelte'
  import type { DiffResponse, LinesDiff } from '../../types'
  import DiffCol from './DiffCol.svelte'
  import { filepathFromDialog } from './scripts'
  import DiffColHeader from './DiffColHeader.svelte'
  import { debounce } from '../../utils'
  import DiffColFooter from './DiffColFooter.svelte'
  import OpHeaderCol from './OpHeaderCol.svelte'
  import OpCol from './OpCol.svelte'
  import { DIFF_LINE_HEIGHT } from './consts'
  import OpFooterCol from './OpFooterCol.svelte'

  let oldFilepath: string = $state('')
  let newFilepath: string = $state('')

  let oldCharset: string = $state('')
  let newCharset: string = $state('')

  let linesDiffs: LinesDiff[] = $state([])
  let focusedLinesDiffIndex: number | null = $state(null)

  let showsFileHandler: boolean = $state(true)

  // todo: not equal diffs in linesDiffs can changed to be 'equal'
  // const linesDiffsDiffOnly: number[] = $derived(
  //   linesDiffs
  //     .map((x, i) => (x.diffKind !== 'equal' ? i : undefined))
  //     .filter((x) => x !== undefined)
  // )
  const prevLinesDiffIndex: number = $derived.by(() => {
    if (focusedLinesDiffIndex === null) return 0
    if (focusedLinesDiffIndex === 0) return focusedLinesDiffIndex
    const foundIndex = linesDiffs.findLastIndex(
      (x, i) => i < focusedLinesDiffIndex! && x.diffKind !== 'equal'
    )
    return 0 <= foundIndex ? foundIndex : focusedLinesDiffIndex
  })
  const nextLinesDiffIndex: number = $derived.by(() => {
    if (focusedLinesDiffIndex === null) return 0
    if (focusedLinesDiffIndex === linesDiffs.length - 1) return focusedLinesDiffIndex
    const foundIndex = linesDiffs.findIndex(
      (x, i) => focusedLinesDiffIndex! < i && x.diffKind !== 'equal'
    )
    return 0 <= foundIndex ? foundIndex : focusedLinesDiffIndex
  })

  const isCompletelyEqual = $derived(!linesDiffs.some((x) => x.diffKind !== 'equal'))

  const diff = async () => {
    invoke('diff_filepaths', { old: oldFilepath, new: newFilepath })
      .then((res: unknown) => {
        console.log(res) // todo

        const diffResponse = res as DiffResponse
        linesDiffs = diffResponse.linesDiffs
        oldCharset = diffResponse.oldCharset
        newCharset = diffResponse.newCharset

        showsFileHandler = false
      })
      .catch((error: unknown) => {
        console.error(error)
        return
      })
  }

  const filepathsOnChange = (oldValue: string, newValue: string) => {
    oldFilepath = oldValue
    newFilepath = newValue
    diff()
  }

  const resetOnClick = () => {
    oldFilepath = ''
    newFilepath = ''
    showsFileHandler = true
    linesDiffs = []
    oldCharset = ''
    newCharset = ''
  }

  const onKeyDown = (
    e: KeyboardEvent & {
      currentTarget: EventTarget & HTMLDivElement
    }
  ) => {
    switch (e.key) {
      case 'F7': {
        focusedLinesDiffIndex = prevLinesDiffIndex
        break
      }
      case 'F8': {
        focusedLinesDiffIndex = nextLinesDiffIndex
        break
      }
      default:
    }
  }

  const linesDiffReplaceOnClick = (linesDiffIndex: number) => {
    const x = linesDiffs[linesDiffIndex]
    x.diffKind = 'equal'
    x.newLines = x.oldLines
    linesDiffs[linesDiffIndex] = x
    if (focusedLinesDiffIndex === linesDiffIndex) focusedLinesDiffIndex = null
  }
</script>

<div class="keyboard-listener" onkeydown={onKeyDown} role="button" tabindex="0">
  <h2>Diff</h2>
  {#if showsFileHandler}
    {#key [oldFilepath, newFilepath]}
      <FileHandle {oldFilepath} {newFilepath} {filepathsOnChange} />
    {/key}
  {/if}
  <label
    ><input
      type="checkbox"
      bind:checked={showsFileHandler}
      disabled={linesDiffs.length === 0}
    />L</label
  >
  <button onclick={resetOnClick}>Reset</button>

  {#if 0 < linesDiffs.length}
    <div class="row header">
      <div class="col diff">
        <DiffColHeader
          oldOrNew="old"
          filepath={oldFilepath}
          {isCompletelyEqual}
          filepathFromDialogOnClick={async () => {
            const filepath = await filepathFromDialog()
            if (filepath === null) return
            oldFilepath = filepath
            diff()
          }}
        />
      </div>
      <div class="col op">
        <OpHeaderCol />
      </div>
      <div class="col diff">
        <DiffColHeader
          oldOrNew="new"
          filepath={newFilepath}
          {isCompletelyEqual}
          filepathFromDialogOnClick={async () => {
            const filepath = await filepathFromDialog()
            if (filepath === null) return
            newFilepath = filepath
            diff()
          }}
        />
      </div>
    </div>
    <div class="row content" style={`--line-height: ${DIFF_LINE_HEIGHT};`}>
      <div class="col diff">
        {#key focusedLinesDiffIndex}
          <DiffCol oldOrNew="old" {linesDiffs} {focusedLinesDiffIndex} />
        {/key}
      </div>
      <div class="col op">
        <OpCol {linesDiffs} {focusedLinesDiffIndex} replaceOnClick={linesDiffReplaceOnClick} />
      </div>
      <div class="col diff">
        {#key focusedLinesDiffIndex}
          <DiffCol oldOrNew="new" {linesDiffs} {focusedLinesDiffIndex} />
        {/key}
      </div>
    </div>
    <div class="row footer">
      <div class="col diff">
        <DiffColFooter oldOrNew="old" charset={oldCharset} />
      </div>
      <div class="col op">
        <OpFooterCol />
      </div>
      <div class="col diff">
        <DiffColFooter oldOrNew="new" charset={newCharset} />
      </div>
    </div>
  {/if}
</div>

<style>
  .keyboard-listener:focus {
    outline: none;
    border: none;
    box-shadow: none;
    /* min-height: 0; */
  }

  .row.header {
    height: 2.7rem;
  }

  .row.content {
    height: 75vh;
    overflow-x: hidden;
    overflow-y: scroll;
  }

  .row.footer {
    height: 1.5rem;
  }

  .col.diff,
  .col.op {
    /* adjust x scrollbar */
    /* min-width: 0; */
    display: flex;
    flex-direction: column;
    height: max-content;
    overflow: hidden;
  }

  .col.op {
    flex-grow: 0;
    flex-basis: 1.4rem;
    background-color: black;
    color: white;
  }
</style>
