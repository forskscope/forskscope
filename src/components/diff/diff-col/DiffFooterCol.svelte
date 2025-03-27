<script lang="ts">
  import { T } from '../../../stores/settings/translation.svelte'

  const {
    charset,
    isCompletelyEqual,
    saveAsOnClick,
  }: {
    charset: string
    isCompletelyEqual: boolean
    saveAsOnClick: (() => Promise<void>) | undefined
  } = $props()
</script>

<div>
  <span>{charset} </span>

  {#if isCompletelyEqual}
    <span>({T('Completely equal')})</span>
  {/if}

  <span>
    {#if saveAsOnClick}
      <button onclick={saveAsOnClick}>{T('Save as')}</button>
    {/if}
  </span>
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
