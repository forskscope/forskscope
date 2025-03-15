<script lang="ts">
  const {
    charset,
    saveAsOnClick,
  }: {
    charset: string
    saveAsOnClick: (() => Promise<void>) | undefined
  } = $props()
</script>

<div>
  <span>{charset}</span>
  {#if saveAsOnClick}
    <button onclick={saveAsOnClick}>Save as</button>
  {/if}
</div>

<style>
  div {
    height: 100%;
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  button {
    padding: 0 0.6rem;
  }
</style>
