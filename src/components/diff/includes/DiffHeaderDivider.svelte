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

<div>
  <button onclick={prevDiffFocus}><MoveUp /></button>
  <button onclick={nextDiffFocus}><MoveDown /></button>
</div>

<style>
  div {
    width: 100%;
    height: 100%;
    overflow-y: scroll;
    display: inline-flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    gap: 0.3rem;
  }

  button {
    padding: 0.1rem;
    font-size: 0.9rem;
  }
</style>
