<script lang="ts">
  import { T } from '../../../../../stores/settings/translation.svelte'
  import ExplorerPaneRow from '../template/ExplorerPaneRow.svelte'

  const { showsHumanReadableSize }: { showsHumanReadableSize: boolean } = $props()
</script>

<ExplorerPaneRow mainClass="headers" {showsHumanReadableSize}>
  {#snippet fileStatus()}{/snippet}
  {#snippet fileIcon()}{/snippet}
  {#snippet fileName()}{T('Name')}{/snippet}
  {#snippet fileHumanReadableSize()}{T('Size')}{/snippet}
  {#snippet fileBytesSize()}{T('Size')}{/snippet}
  {#snippet fileLastModified()}{T('Last modified')}{/snippet}
</ExplorerPaneRow>
