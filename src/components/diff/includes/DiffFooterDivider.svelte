<script lang="ts">
  import { Menu, Undo } from 'lucide-svelte'

  const {
    toggleCharsDiffs,
    undoMergeOnClick,
  }: { toggleCharsDiffs: () => void; undoMergeOnClick: () => void } = $props()

  let showsMenus: boolean = $state(false)
</script>

<div class="menus-wrapper">
  <button class={`toggle ${showsMenus ? 'opened' : ''}`} onclick={() => (showsMenus = !showsMenus)}
    ><Menu /></button
  >

  <div class={`menus ${showsMenus ? 'active' : 'd-none'}`}>
    <button onclick={toggleCharsDiffs}>chars diff</button>
    <button onclick={undoMergeOnClick}><Undo /> undo merge</button>
  </div>
</div>

<style>
  .menus-wrapper {
    position: relative;
    display: flex;
    justify-content: center;
    align-items: center;
  }

  .menus {
    position: absolute;
    bottom: 2.2rem;
    width: fit-content;
    height: fit-content;
    opacity: 0.87;
  }

  .toggle {
    padding: 0.2rem;
    display: inline-flex;
    justify-content: center;
    align-items: center;
    transition: transform 0.3s ease-out;
  }

  .menus-wrapper:has(.menus.active) .toggle {
    transform: rotate(90deg);
  }
</style>
