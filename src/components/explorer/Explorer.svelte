<script lang="ts">
  import View from '../../layouts/default/view/View.svelte'
  import { exploreIsActive } from '../../stores/compareSets.svelte'
  import ExplorerContent from './content/ExplorerContent.svelte'
  import ExplorerFooter from './includes/ExplorerFooter.svelte'
  import ExplorerHeader from './includes/ExplorerHeader.svelte'
  import ExplorerFooterDivider from './includes/ExplorerFooterDivider.svelte'
  import { initialize } from '../../stores/explorer.svelte'
  import { onMount } from 'svelte'
  import ExplorerContentDivider from './content/ExplorerContentDivider.svelte'

  const visible: boolean = $state(exploreIsActive())

  let showsHumanReadableSize: boolean = $state(true)

  onMount(() => {
    initialize()
  })

  const toggleShowsHumanReadableSize = () => {
    showsHumanReadableSize = !showsHumanReadableSize
  }
</script>

<View mainClass="explorer" {visible}>
  {#snippet leftHeader()}
    <ExplorerHeader oldOrNew="old" />
  {/snippet}
  {#snippet headerDivider()}{/snippet}
  {#snippet rightHeader()}
    <ExplorerHeader oldOrNew="new" />
  {/snippet}

  {#snippet leftContent()}
    <ExplorerContent oldOrNew="old" {showsHumanReadableSize} />
  {/snippet}
  {#snippet contentDivider()}
    <ExplorerContentDivider />
  {/snippet}
  {#snippet rightContent()}
    <ExplorerContent oldOrNew="new" {showsHumanReadableSize} />
  {/snippet}

  {#snippet leftFooter()}
    <ExplorerFooter oldOrNew="old" />
  {/snippet}
  {#snippet footerDivider()}
    <ExplorerFooterDivider {showsHumanReadableSize} {toggleShowsHumanReadableSize} />
  {/snippet}
  {#snippet rightFooter()}
    <ExplorerFooter oldOrNew="new" />
  {/snippet}
</View>
