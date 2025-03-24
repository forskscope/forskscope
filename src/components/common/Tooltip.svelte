<script lang="ts">
  import type { TooltipPosition } from '../../types/Tooltip.svelte'

  let {
    messages,
    position,
    children,
  }: {
    messages: string
    position: TooltipPosition
    children: any
  } = $props()
</script>

<div class="tooltip-container">
  {@render children()}
  <div class={`tooltip ${position}`}>{messages}</div>
</div>

<style>
  .tooltip-container {
    position: relative;
    display: inline-flex;
  }

  .tooltip {
    position: absolute;
    min-width: 5.7rem;
    max-width: fit-content;
    padding: 0.44rem;
    background-color: #636363ce;
    color: #ffffff;
    font-size: 0.63rem;
    text-align: center;
    opacity: 0.2;
    visibility: hidden;
    z-index: 1;
    transition:
      opacity 0.3s,
      visibility 0.3s;
  }

  .tooltip-container:hover .tooltip {
    opacity: 1;
    visibility: visible;
  }

  .tooltip.top {
    bottom: 100%;
    left: 50%;
    transform: translateX(-50%);
    margin-bottom: 0.3rem;
  }
  .tooltip.right {
    left: 100%;
    top: 50%;
    transform: translateY(-50%);
    margin-left: 0.3rem;
  }
  .tooltip.bottom {
    top: 100%;
    left: 50%;
    transform: translateX(-50%);
    margin-top: 0.3rem;
  }
  .tooltip.left {
    right: 100%;
    top: 50%;
    transform: translateY(-50%);
    margin-right: 0.3rem;
  }
</style>
