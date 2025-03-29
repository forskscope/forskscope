<script lang="ts">
  import './style.scss'

  const {
    mainClass,
    customStyle,
    visible,
    scrollSyncs,
    leftHeader,
    headerDivider,
    rightHeader,
    leftContent,
    contentDivider,
    rightContent,
    leftFooter,
    footerDivider,
    rightFooter,
  }: {
    mainClass: string
    customStyle?: string
    visible: boolean
    scrollSyncs: boolean
    leftHeader: any
    headerDivider: any
    rightHeader: any
    leftContent: any
    contentDivider: any
    rightContent: any
    leftFooter: any
    footerDivider: any
    rightFooter: any
  } = $props()

  let leftContentEl: HTMLDivElement | undefined
  let contentDividerEl: HTMLDivElement | undefined
  let rightContentEl: HTMLDivElement | undefined

  let scrollTimeout: number | null = null
  const scrollSync = (
    e: UIEvent & {
      currentTarget: EventTarget & HTMLDivElement
    },
    el: HTMLDivElement | undefined
  ) => {
    if (!el || !scrollSyncs) return

    if (scrollTimeout !== null) cancelAnimationFrame(scrollTimeout)
    const left: number = e.currentTarget.scrollLeft
    const top: number = e.currentTarget.scrollTop
    // smooth scroll
    scrollTimeout = requestAnimationFrame(() => {
      el.scrollTo(left, top)
      if (contentDividerEl) contentDividerEl.scroll(left, top)
    })
  }
</script>

<div
  class={`${mainClass} view-container ${visible ? '' : 'd-none'}`}
  style={customStyle ? customStyle : ''}
>
  <div class="view-header">
    <div class="content">
      {@render leftHeader()}
    </div>
    <div class="divider">
      {@render headerDivider()}
    </div>
    <div class="content">
      {@render rightHeader()}
    </div>
  </div>

  <div class="view-body">
    <div class="content" bind:this={leftContentEl} onscroll={(e) => scrollSync(e, rightContentEl)}>
      {@render leftContent()}
    </div>
    <div class="divider" bind:this={contentDividerEl}>
      {@render contentDivider()}
    </div>
    <div class="content" bind:this={rightContentEl} onscroll={(e) => scrollSync(e, leftContentEl)}>
      {@render rightContent()}
    </div>
  </div>

  <div class="view-footer">
    <div class="content">
      {@render leftFooter()}
    </div>
    <div class="divider">
      {@render footerDivider()}
    </div>
    <div class="content">
      {@render rightFooter()}
    </div>
  </div>
</div>
