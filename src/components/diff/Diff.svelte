<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import FileHandle from './FileHandle.svelte'
  import type { DiffResponse, LinesDiff } from '../../types'
  import DiffCol from './DiffCol.svelte'
  import { filepathFromDialog } from './scripts'
  import DiffColHeader from './DiffColHeader.svelte'
  import { debounce } from '../../utils'
  import DiffColFooter from './DiffColFooter.svelte'

  let oldFilepath: string = $state('')
  let newFilepath: string = $state('')

  let showsFileHandler: boolean = $state(true)

  let linesDiffs: LinesDiff[] = $state([])
  let focusedLinesDiffIndex: number | null = $state(null)

  let oldContent: HTMLDivElement | null = $state(null)
  let newContent: HTMLDivElement | null = $state(null)
  let oldCharset: string = $state('')
  let newCharset: string = $state('')

  const linesDiffIndexDiffOnly: number[] = $derived(
    linesDiffs
      .map((x, i) => (x.diffKind !== 'equal' ? i : undefined))
      .filter((x) => x !== undefined)
  )

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

  const oldContentOnScroll = (
    e: UIEvent & {
      currentTarget: EventTarget & HTMLDivElement
    }
  ) => {
    if (e.currentTarget.scrollLeft !== oldContent!.scrollLeft) return
    const scrollTop = e.currentTarget.scrollTop
    debounce(() => {
      newContent!.scrollTop = scrollTop
    }, 10)
  }
  const newContentOnScroll = (
    e: UIEvent & {
      currentTarget: EventTarget & HTMLDivElement
    }
  ) => {
    if (e.currentTarget.scrollLeft !== newContent!.scrollLeft) return
    const scrollTop = e.currentTarget.scrollTop
    debounce(() => {
      oldContent!.scrollTop = scrollTop
    }, 10)
  }
</script>

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

{#if 0 < linesDiffIndexDiffOnly.length}
  <select bind:value={focusedLinesDiffIndex}>
    {#each linesDiffIndexDiffOnly as item}
      <option value={item}>{item}</option>
    {/each}
  </select>
{/if}

{#if 0 < linesDiffs.length}
  <div class="row">
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
      <div class="content" onscroll={oldContentOnScroll} bind:this={oldContent}>
        {#key focusedLinesDiffIndex}
          <DiffCol oldOrNew="old" {linesDiffs} {focusedLinesDiffIndex} />
        {/key}
      </div>
      <DiffColFooter oldOrNew="old" charset={oldCharset} />
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
      <div class="content" onscroll={newContentOnScroll} bind:this={newContent}>
        {#key focusedLinesDiffIndex}
          <DiffCol oldOrNew="new" {linesDiffs} {focusedLinesDiffIndex} />
        {/key}
      </div>
      <DiffColFooter oldOrNew="new" charset={newCharset} />
    </div>
  </div>
{/if}

<style>
  .diff {
    width: 100%;
    /* adjust x scrollbar */
    min-width: 0;
    /* todo: fit height to window */
    height: calc(100vh - 5.7rem);
    display: flex;
    flex-direction: column;
  }
  .diff .content {
    overflow: auto;
  }
</style>
