<script lang="ts">
  import { getActiveCompareSet } from '../../../../stores/tabs.svelte'
  import DiffHeader from '../templates/DiffHeader.svelte'
</script>

<DiffHeader oldOrNew="old" filepath={getActiveCompareSet()!.old.filepath} />
