<script lang="ts">
  import Tooltip from '../components/common/Tooltip.svelte'

  let { toggleSettings }: { toggleSettings: () => void } = $props()
</script>

<div>
  <h1>Patch Hygge</h1>

  <Tooltip position="bottom" messages="settings">
    <button
      onclick={() => {
        toggleSettings()
      }}
      >⚙️
    </button>
  </Tooltip>
</div>

<style>
  div {
    position: fixed;
    top: 0.2rem;
    right: 0.7rem;
    display: flex;
    align-items: center;
    font-size: 1.1rem;
    gap: 0.4rem;
    z-index: 1001;
  }

  h1 {
    font-size: 0.9rem;
    opacity: 0.4;
  }

  button {
    padding: 0 0.3rem;
    opacity: 0.63;
  }
</style>
