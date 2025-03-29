<script lang="ts">
  import './style.scss'

  const {
    visible,
    syncsScroll,
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
    visible: boolean
    syncsScroll: boolean
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
  let rightContentEl: HTMLDivElement | undefined

  const syncScroll = (
    e: UIEvent & {
      currentTarget: EventTarget & HTMLDivElement
    },
    el: HTMLDivElement | undefined
  ) => {
    if (!el || !syncsScroll) return
    const left: number = e.currentTarget.scrollLeft
    const top: number = e.currentTarget.scrollTop
    el.scrollTo(left, top)
  }
</script>

<div class={`view-container ${visible ? '' : 'd-none'}`}>
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
    <div class="content" bind:this={leftContentEl} onscroll={(e) => syncScroll(e, rightContentEl)}>
      {@render leftContent()}
    </div>
    <div class="divider">
      {@render contentDivider()}
    </div>
    <div class="content" bind:this={rightContentEl} onscroll={(e) => syncScroll(e, leftContentEl)}>
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
