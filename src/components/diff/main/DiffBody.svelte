<script lang="ts">
  import DiffContent from './DiffContent.svelte'
  import DiffContentDivider from './DiffContentDivider.svelte'

  let scrollLeft: number = $state(0)
  let scrollTop: number = $state(0)

  const onScroll = (_scrollLeft: number, _scrollTop: number) => {
    scrollLeft = _scrollLeft
    scrollTop = _scrollTop
  }
</script>

<DiffContent oldOrNew="old" {scrollLeft} {scrollTop} {onScroll} />
<DiffContentDivider />
<DiffContent oldOrNew="new" {scrollLeft} {scrollTop} {onScroll} />
