<script lang="ts">
  import type { BackendCommandResult, CharsDiffResponse, LinesDiffResponse } from '../../../types'
  import DiffContent from './DiffContent.svelte'
  import DiffContentDivider from './DiffContentDivider.svelte'
  import { invokeWithGuard } from '../../../utils/backend.svelte'

  const {
    linesDiffResponse,
    showsCharsDiffs,
    focusedLinesDiffIndex,
  }: {
    linesDiffResponse: LinesDiffResponse
    showsCharsDiffs: boolean
    focusedLinesDiffIndex: number | null
  } = $props()

  let charsDiffResponse: CharsDiffResponse | null = $state(null)

  let scrollLeft: number = $state(0)
  let scrollTop: number = $state(0)

  $effect(() => {
    if (showsCharsDiffs) {
      if (charsDiffResponse === null) diffChars()
    }
  })

  const diffChars = async () => {
    const replaceLinesDiffs = linesDiffResponse.diffs.filter((x) => x.diffKind === 'replace')
    if (replaceLinesDiffs.length === 0) return

    const res: BackendCommandResult = await invokeWithGuard('diff_chars', {
      linesDiffs: replaceLinesDiffs,
    })

    charsDiffResponse = res.response as CharsDiffResponse
  }

  const onScroll = (_scrollLeft: number, _scrollTop: number) => {
    scrollLeft = _scrollLeft
    scrollTop = _scrollTop
  }
</script>

<DiffContent
  oldOrNew="old"
  {linesDiffResponse}
  {charsDiffResponse}
  {showsCharsDiffs}
  {focusedLinesDiffIndex}
  {scrollLeft}
  {scrollTop}
  {onScroll}
/>
<DiffContentDivider />
<DiffContent
  oldOrNew="new"
  {linesDiffResponse}
  {charsDiffResponse}
  {showsCharsDiffs}
  {focusedLinesDiffIndex}
  {scrollLeft}
  {scrollTop}
  {onScroll}
/>
