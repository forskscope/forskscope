<script lang="ts">
  import { ArrowLeftCircle, ArrowRightCircle, HardDrive } from 'lucide-svelte'
  import type { OldOrNew } from '../../../types/compareSets'
  import Tooltip from '../../common/Tooltip.svelte'
  import { openWithFileManager } from '../../../utils/file.svelte'
  import { T } from '../../../stores/settings/translation.svelte'
  import { newListDirResponse, oldListDirResponse, syncDir } from '../../../stores/explorer.svelte'

  const {
    oldOrNew,
  }: {
    oldOrNew: OldOrNew
  } = $props()

  const listDirResponse = $derived(oldOrNew === 'old' ? $oldListDirResponse : $newListDirResponse)

  const currentDir: string = $derived(listDirResponse !== null ? listDirResponse.currentDir : '')
</script>

<div class={`buttons ${oldOrNew}`}>
  <Tooltip position="top" messages={T('Sync dir pos')}>
    <button class="sync-dir" onclick={() => syncDir(oldOrNew)}>
      {#if oldOrNew === 'old'}
        <ArrowRightCircle />
      {:else}
        <ArrowLeftCircle />
      {/if}
    </button>
  </Tooltip>
  <Tooltip position="top" messages={T('Run file manager')}>
    <button class="file-manager" onclick={() => openWithFileManager(currentDir)}>
      <HardDrive />
    </button>
  </Tooltip>
</div>

<style>
  .buttons {
    display: flex;
    align-items: center;
    gap: 0.6rem;
  }

  .old {
    margin-right: 1.4rem;
    flex-direction: row-reverse;
  }
  .new {
    margin-left: 1.4rem;
    flex-direction: row;
  }

  button {
    padding: 0.2rem 0.4rem;
    font-size: 1.2rem;
  }
</style>
