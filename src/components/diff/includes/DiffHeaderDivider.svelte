<script lang="ts">
  import { MoveDown, MoveUp } from 'lucide-svelte'

  const {
    focusedLinesDiffIndexOnChange,
  }: { focusedLinesDiffIndexOnChange: (delta: number) => void } = $props()

  const prevDiffFocus = () => {
    focusedLinesDiffIndexOnChange(-1)
  }

  const nextDiffFocus = () => {
    focusedLinesDiffIndexOnChange(1)
  }
</script>

<div class="menus-wrapper">
  <div class="menus">
    <button onclick={prevDiffFocus}><MoveUp /></button>
    <button onclick={nextDiffFocus}><MoveDown /></button>
  </div>
</div>

<style>
  .menus-wrapper {
    position: relative;
    display: flex;
    justify-content: center;
  }
  .menus {
    position: relative;
    top: -0.7rem;
    display: inline-flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    gap: 0.3rem;
  }

  button {
    padding: 0.1rem 0.2rem;
    font-size: 0.9rem;
  }
</style>
