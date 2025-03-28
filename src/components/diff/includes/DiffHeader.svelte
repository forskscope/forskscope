<script lang="ts">
  import { getActiveCompareSet } from '../../../stores/tabs.svelte'
</script>

<div class="old-or-new old">
  <div class="filepath" style="background-color: orange;">
    {getActiveCompareSet()?.old.filepath}
  </div>
  <button>!!</button>
</div>
<div class="divider"></div>
<div class="old-or-new new">
  <div class="filepath" style="background-color: green;">{getActiveCompareSet()?.new.filepath}</div>
  <button>!!</button>
</div>
