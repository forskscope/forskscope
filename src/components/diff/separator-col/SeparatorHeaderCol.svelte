<script lang="ts">
  let {
    charsDiffsReady,
    switchOldNewOnClick,
  }: { charsDiffsReady: boolean; switchOldNewOnClick: () => void } = $props()
</script>

<!-- todo: switch new/old -->
<div><button onclick={switchOldNewOnClick} disabled={!charsDiffsReady}>&lt;&gt;</button></div>

<style>
  button {
    padding: 0;
  }
</style>
