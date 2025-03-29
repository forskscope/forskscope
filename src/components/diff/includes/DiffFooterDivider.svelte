<script lang="ts">
  const { toggleCharsDiffs }: { toggleCharsDiffs: () => void } = $props()

  let showsMenus: boolean = $state(false)
</script>

<div class="menus-wrapper">
  <input type="checkbox" bind:checked={showsMenus} />

  {#if showsMenus}
    <div class="menus">
      <button onclick={toggleCharsDiffs}>chars diff</button>
    </div>
  {/if}
</div>

<style>
  .menus-wrapper {
    position: relative;
  }

  .menus {
    position: absolute;
    bottom: 2.2rem;
    width: 80vw;
    height: 2.2rem;
    opacity: 0.87;
  }
</style>
