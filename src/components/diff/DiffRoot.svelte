<script lang="ts">
  import DiffBody from './main/DiffBody.svelte'
  import { onMount } from 'svelte'
  import { invokeWithGuard } from '../../utils/backend.svelte'
  import type { BackendCommandResult, CompareSet, LinesDiffResponse } from '../../types'
  import { removeActiveCompareSet } from '../../stores/tabs.svelte'
  import View from '../../layouts/default/view/View.svelte'
  import DiffOldHeader from './includes/header/DiffOldHeader.svelte'
  import DiffNewHeader from './includes/header/DiffNewHeader.svelte'
  import DiffHeaderDivider from './includes/header/DiffHeaderDivider.svelte'
  import DiffOldFooter from './includes/footer/DiffOldFooter.svelte'
  import DiffNewFooter from './includes/footer/DiffNewFooter.svelte'
  import DiffFooterDivider from './includes/footer/DiffFooterDivider.svelte'

  const { compareSet, visible }: { compareSet: CompareSet; visible: boolean } = $props()

  let linesDiffResponse: LinesDiffResponse | null = $state(null)

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
</script>

{#if linesDiffResponse !== null}
  <View {visible} syncsScroll={true}>
    {#snippet leftHeader()}<DiffOldHeader />{/snippet}
    {#snippet headerDivider()}<DiffHeaderDivider />{/snippet}
    {#snippet rightHeader()}<DiffNewHeader />{/snippet}
    {#snippet leftContent()}<div style="width: 110vw; height: 110vh;">
        aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
      </div>{/snippet}
    {#snippet contentDivider()}b{/snippet}
    {#snippet rightContent()}<div style="width: 110vw; height: 110vh;">
        cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc
      </div>{/snippet}
    {#snippet leftFooter()}<DiffOldFooter linesDiffResponse={linesDiffResponse!} />{/snippet}
    {#snippet footerDivider()}<DiffFooterDivider />{/snippet}
    {#snippet rightFooter()}<DiffNewFooter linesDiffResponse={linesDiffResponse!} />{/snippet}
  </View>
{:else}
  <!-- todo: loading -->
  (...... Loading ......)
{/if}
