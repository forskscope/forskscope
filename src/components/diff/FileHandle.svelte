<script lang="ts">
  import DragDrop from '../common/DragDrop.svelte'
  import { filepathFromDialog } from './scripts'

  const {
    oldFilepath,
    newFilepath,
    filepathsOnChange,
  }: {
    oldFilepath: string
    newFilepath: string
    filepathsOnChange: (oldFilepath: string, newFilepath: string) => void
  } = $props()

  let _oldFilepath: string = $state(oldFilepath)
  let _newFilepath: string = $state(newFilepath)

  const onDrop = (filepaths: string[]) => {
    if (filepaths.length === 0) return
    if (filepaths.length === 1) {
      const updated = filepaths[0]
      if (_oldFilepath && !_newFilepath) {
        _newFilepath = updated
      } else {
        _oldFilepath = updated
      }
    } else {
      _oldFilepath = filepaths[0]
      _newFilepath = filepaths[1]
    }
    filepathsOnChangeIfReady()
  }

  const filepathsOnChangeIfReady = () => {
    if (!_oldFilepath || !_newFilepath) return
    filepathsOnChange(_oldFilepath, _newFilepath)
  }

  const openOldFileOnClick = async () => {
    const filepath = await filepathFromDialog()
    if (!filepath) return
    _oldFilepath = filepath
    filepathsOnChangeIfReady()
  }

  const openNewFileOnClick = async () => {
    const filepath = await filepathFromDialog()
    if (!filepath) return
    _newFilepath = filepath
    filepathsOnChangeIfReady()
  }
</script>

<div class="filedrop">
  <DragDrop {onDrop} />
  <div>
    <button onclick={openOldFileOnClick}>Old file</button>
    <span>{_oldFilepath}</span>
  </div>
  <div>
    <button onclick={openNewFileOnClick}>New file</button>
    <span>{_newFilepath}</span>
  </div>
</div>

<style>
  .filedrop {
    width: 100%;
    height: auto;
    padding: 0.4rem 0;
    background-color: yellow;
    color: #252525;
  }
  .filedrop > div {
    padding: 0.5rem 1.5rem;
    display: flex;
    align-items: center;
  }
  button {
    width: 7.2em;
    margin-right: 1.5rem;
  }
</style>
