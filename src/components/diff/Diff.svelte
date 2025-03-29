<script lang="ts">
  import { onMount } from 'svelte'
  import { invokeWithGuard } from '../../utils/backend.svelte'
  import type {
    BackendCommandResult,
    CharsDiffResponse,
    CompareSet,
    LinesDiffResponse,
  } from '../../types'
  import { removeActiveCompareSet } from '../../stores/tabs.svelte'
  import View from '../../layouts/default/view/View.svelte'
  import DiffHeaderDivider from './includes/DiffHeaderDivider.svelte'
  import DiffContent from './content/DiffContent.svelte'
  import DiffFooterDivider from './includes/DiffFooterDivider.svelte'
  import DiffHeader from './includes/DiffHeader.svelte'
  import DiffFooter from './includes/DiffFooter.svelte'

  const { compareSet, visible }: { compareSet: CompareSet; visible: boolean } = $props()

  let linesDiffResponse: LinesDiffResponse | null = $state(null)
  let charsDiffResponse: CharsDiffResponse | null = $state(null)

  // todo
  let showsCharsDiffs: boolean = $state(false)
  let focusedLinesDiffIndex: number | null = $state(null)

  const oldFilepath: string = $derived(compareSet.old.filepath)
  const newFilepath: string = $derived(compareSet.new.filepath)

  onMount(async () => {
    await diffLines()
  })

  const diffLines = async () => {
    const res: BackendCommandResult = await invokeWithGuard('diff_filepaths', {
      old: oldFilepath,
      new: newFilepath,
    })
    if (res.isError) {
      removeActiveCompareSet()
      return
    }

    linesDiffResponse = res.response as LinesDiffResponse

    focusedLinesDiffIndex = null
  }

  const diffChars = async () => {
    const replaceLinesDiffs = linesDiffResponse!.diffs.filter((x) => x.diffKind === 'replace')
    if (replaceLinesDiffs.length === 0) return

    const res: BackendCommandResult = await invokeWithGuard('diff_chars', {
      linesDiffs: replaceLinesDiffs,
    })

    charsDiffResponse = res.response as CharsDiffResponse

    // todo: move to another ui
    showsCharsDiffs = true
  }
</script>

{#if linesDiffResponse !== null}
  <View {visible} scrollSyncs={true}>
    {#snippet leftHeader()}<DiffHeader oldOrNew="old" {compareSet} />{/snippet}
    {#snippet headerDivider()}
      <!-- todo: <DiffHeaderDivider /> -->
      <button onclick={diffChars}>chars</button>
    {/snippet}
    {#snippet rightHeader()}<DiffHeader oldOrNew="new" {compareSet} />{/snippet}
    {#snippet leftContent()}
      <DiffContent
        oldOrNew="old"
        linesDiffResponse={linesDiffResponse!}
        {charsDiffResponse}
        {showsCharsDiffs}
        {focusedLinesDiffIndex}
      />
    {/snippet}
    {#snippet contentDivider()}b{/snippet}
    {#snippet rightContent()}
      <DiffContent
        oldOrNew="new"
        linesDiffResponse={linesDiffResponse!}
        {charsDiffResponse}
        {showsCharsDiffs}
        {focusedLinesDiffIndex}
      />
    {/snippet}
    {#snippet leftFooter()}<DiffFooter oldOrNew="old" {linesDiffResponse} />{/snippet}
    {#snippet footerDivider()}<DiffFooterDivider />{/snippet}
    {#snippet rightFooter()}<DiffFooter oldOrNew="new" {linesDiffResponse} />{/snippet}
  </View>
{:else}
  <!-- todo: loading -->
  (...... Loading ......)
{/if}
