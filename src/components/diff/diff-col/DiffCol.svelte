<script lang="ts">
  import type { LinesDiff, OldOrNew } from '../../../types'
  import DiffColEditorLayer from './DiffColEditorLayer.svelte'
  import DiffColEffectLayer from './DiffColEffectLayer.svelte'
  import './styles.scss'

  const {
    oldOrNew,
    linesDiffs,
    focusedLinesDiffIndex,
  }: {
    oldOrNew: OldOrNew
    linesDiffs: LinesDiff[]
    focusedLinesDiffIndex: number | null
  } = $props()
</script>

<div class="layers">
  <div class="layer editor">
    <DiffColEditorLayer {oldOrNew} {linesDiffs} />
  </div>
  <div class="layer effect">
    <DiffColEffectLayer {oldOrNew} {linesDiffs} {focusedLinesDiffIndex} />
  </div>
</div>
