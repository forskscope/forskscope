<script lang="ts">
  import { onMount } from 'svelte'
  import type { OldOrNew } from '../../../types'

  const {
    oldOrNew,
    scrollLeft,
    scrollTop,
    onScroll,
  }: {
    oldOrNew: OldOrNew
    scrollLeft: number
    scrollTop: number
    onScroll: (scrollLeft: number, scrollTop: number) => void
  } = $props()

  let wrapper: HTMLDivElement | undefined

  $effect(() => {
    if (!isNaN(scrollTop) && !isNaN(scrollLeft)) {
      if (!wrapper) return
      wrapper.scrollTo(scrollLeft, scrollTop)
    }
  })
</script>

<div
  class={`diff-content ${oldOrNew}`}
  style="background: grey;"
  bind:this={wrapper}
  onscroll={(e) => {
    const t = e.currentTarget
    onScroll(t.scrollLeft, t.scrollTop)
  }}
>
  {oldOrNew}
  <div class="overflown">kjlkdjflksajfslk</div>
</div>

<style>
  .overflown {
    width: 110vw;
    height: 110vh;
  }
</style>
