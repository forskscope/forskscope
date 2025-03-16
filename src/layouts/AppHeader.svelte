<script lang="ts">
  let { toggleSettings }: { toggleSettings: () => void } = $props()
</script>

<div>
  <h1>Patch Hygge</h1>

  <button
    onclick={() => {
      toggleSettings()
    }}
    >⚙️
  </button>
</div>

<style>
  div {
    position: fixed;
    top: 0.2rem;
    right: 0.7rem;
    display: flex;
    align-items: center;
    font-size: 1.1rem;
    opacity: 0.4;
    gap: 0.4rem;
    z-index: 1001;
  }

  h1 {
    font-size: 0.9rem;
  }

  button {
    padding: 0 0.3rem;
  }
</style>
