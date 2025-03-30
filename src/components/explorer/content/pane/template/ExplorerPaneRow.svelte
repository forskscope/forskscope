<script lang="ts">
  const {
    mainClass,
    selected,
    showsHumanReadableSize,
    rowOnClick,
    rowOnDblClick,
    fileStatus,
    fileIcon,
    fileName,
    fileHumanReadableSize,
    fileBytesSize,
    fileLastModified,
  }: {
    mainClass?: string
    selected?: boolean
    showsHumanReadableSize: boolean
    rowOnClick?: Function
    rowOnDblClick?: Function
    fileStatus: any
    fileIcon: any
    fileName: any
    fileHumanReadableSize: any
    fileBytesSize: any
    fileLastModified: any
  } = $props()
</script>

<div
  class={`row ${mainClass ? mainClass : ''}`}
  role="button"
  tabindex="0"
  onclick={() => {
    if (rowOnClick) {
      rowOnClick()
    }
  }}
  ondblclick={() => {
    if (rowOnDblClick) {
      rowOnDblClick()
    }
  }}
  onkeydown={(event) => {
    switch (event.key) {
      case ' ': {
        if (rowOnClick) {
          rowOnClick()
        }
        break
      }
      case 'Enter': {
        if (rowOnDblClick) {
          // change dir or directly compare
          rowOnDblClick()
        } else if (rowOnClick) {
          // default action
          rowOnClick()
        }
        break
      }
      default:
    }
  }}
>
  <div class="col status">
    {@render fileStatus()}
  </div>
  <div class="col icon">
    {@render fileIcon()}
  </div>
  <div class={`col name ${selected ? 'selected' : ''}`}>
    {@render fileName()}
  </div>
  {#if showsHumanReadableSize}
    <div class="col size human-readable">
      {@render fileHumanReadableSize()}
    </div>
  {:else}
    <div class="col size bytes">
      {@render fileBytesSize()}
    </div>
  {/if}
  <div class="col last-modified">
    {@render fileLastModified()}
  </div>
</div>

<style>
  .row {
    flex: 1;
  }

  .row:not(.headers):not(.footer) {
    cursor: pointer;
  }

  .row.headers {
    /* todo */
    position: sticky;
  }

  .row:not(.headers):not(.footer) .col.name {
    position: sticky;
  }

  .col:not(.name) {
    flex: unset;
  }

  .status,
  .icon {
    width: 1.2rem;
  }

  .name {
    flex: 1 1 0;
  }
</style>
