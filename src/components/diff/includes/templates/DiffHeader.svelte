<script lang="ts">
  import type { OldOrNew } from '../../../../types'

  const { oldOrNew, filepath }: { oldOrNew: OldOrNew; filepath: string } = $props()
</script>

<div class={`old-or-new ${oldOrNew}`}>
  <div class="filepath">{filepath}</div>
  <button>!!</button>
</div>

<style>
  .old-or-new {
    display: flex;
    flex: 1;
  }
  .filepath {
    flex: 1;
    /* todo: split into divs to emphasize filename */
    overflow: hidden;
    text-overflow: ellipsis;
  }
  button {
    width: 2.2rem;
    padding: 0.3rem;
  }
</style>
