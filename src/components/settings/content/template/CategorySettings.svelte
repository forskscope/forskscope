<script lang="ts">
  import { T } from '../../../../stores/settings/translation.svelte'
  import type { SettingsSelector } from '../../../../types/settings'

  const { selectors }: { selectors: SettingsSelector[] } = $props()
</script>

{#each selectors as selector}
  <div class="setting">
    <h3><selector.icon /> {T(selector.title)}</h3>
    <div>
      {#if selector.type === 'radio'}
        {#each selector.options as option}
          <label>
            <input
              type="radio"
              name={selector.groupName}
              value={option}
              onchange={(e) => {
                selector.onchange(e.currentTarget.value)
              }}
              checked={option === selector.defaultValue}
            />
            {T(option.replace(selector.valueSuffix, ''))}
          </label>
        {/each}
      {:else if selector.type === 'select'}
        <select
          onchange={(e) => {
            selector.onchange(e.currentTarget.value)
          }}
        >
          {#each selector.options as option}
            <option value={option} selected={option === selector.defaultValue}>
              {selector.optionLabels ? T(selector.optionLabels[option]) : T(option)}
            </option>
          {/each}
        </select>
      {:else if selector.type === 'number'}
        <input
          type="number"
          step={selector.step}
          min={selector.min}
          max={selector.max}
          value={selector.defaultValue}
          onchange={(e) => {
            selector.onchange(e.currentTarget.value)
          }}
        />
      {/if}
    </div>
  </div>
{/each}

<style>
  h3 {
    margin-bottom: 0.3rem;
  }
</style>
