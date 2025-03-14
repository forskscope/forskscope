<script lang="ts">
  import { _toasts } from '../../stores/Toast.svelte.js'
</script>

<div class="toasts-container">
  {#each _toasts as toast}
    <div class={`toast ${toast.type}`}>{toast.messages}</div>
  {/each}
</div>

<style>
  .toasts-container {
    position: fixed;
    right: 2.4rem;
    top: 0.6rem;
    width: 14.4rem;
    max-width: calc(90vw - 2.4rem);
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    z-index: 30000;
  }

  @keyframes toast-fade-in {
    from {
      opacity: 0;
      transform: translateY(1.2rem);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .toast {
    width: 100%;
    padding: 0.5rem 0.8rem;
    border-radius: 0.04rem;
    word-wrap: break-word;
    opacity: 0;
    transform: translateY(1.2rem);
    animation: toast-fade-in 1.5s ease forwards;
  }
  /* todo: color vars */
  .toast.info {
    background-color: #636363;
    color: #ffffff;
  }
  .toast.success {
    background-color: #2aabb7;
    color: #ffffff;
  }
  .toast.error {
    background-color: #d20f63;
    color: #ffffff;
  }
</style>
