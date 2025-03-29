<script lang="ts">
  import { getActiveCompareSet } from '../../../../stores/tabs.svelte'
  import DiffHeader from '../templates/DiffHeader.svelte'
</script>

<DiffHeader oldOrNew="new" filepath={getActiveCompareSet()!.new.filepath} />
