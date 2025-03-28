<script lang="ts">
  import DiffBody from './main/DiffBody.svelte'
  import DiffFooter from './includes/DiffFooter.svelte'
  import DiffHeader from './includes/DiffHeader.svelte'
  import './style.scss'
</script>

<div class="diff-main">
  <div class="diff-header" style="background: blue;">
    <DiffHeader />
  </div>
  <div class="diff-body" style="background: yellow;">
    <DiffBody />
  </div>
  <div class="diff-footer" style="background: red;">
    <DiffFooter />
  </div>
</div>
