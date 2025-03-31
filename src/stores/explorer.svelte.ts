import { get, writable, type Writable } from "svelte/store"
import type { ListDirResponse } from "../types/file"
import { pathJoin } from "../utils/file.svelte"
import type { OldOrNew } from "../types/compareSets"
import { invokeWithGuard } from "../utils/backend.svelte"
import { pushCompareSetFromFilepaths } from "./compareSets.svelte"
import { errorToast } from "./Toast.svelte"
import { T } from "./settings/translation.svelte"
import type { DigestDiff } from "../types/explorer"
import type { BackendCommandResult } from "../types/backend"

export const oldListDirResponse: Writable<ListDirResponse | null> = writable(null)
export const newListDirResponse: Writable<ListDirResponse | null> = writable(null)

export const fileDigestDiffs: Writable<DigestDiff[]> = writable([])
export const dirDigestDiffs: Writable<DigestDiff[]> = writable([])

export const oldSelectedFileIndex: Writable<number | null> = writable(null)
export const newSelectedFileIndex: Writable<number | null> = writable(null)

export const changeDirProcessing: Writable<boolean> = writable(false)

export const initialize = async () => {
    const listDirResponse = await listDir('')
    oldListDirResponse.set(listDirResponse)
    newListDirResponse.set(listDirResponse)
}

// todo: export if necessary on explorer pane footer detail shown
// export const listDirResponse = (oldOrNew: OldOrNew): ListDirResponse | null => {
const listDirResponse = (oldOrNew: OldOrNew): ListDirResponse | null => {
    return oldOrNew === "old" ? get(oldListDirResponse) : get(newListDirResponse)
}

export const selectFile = (oldOrNew: OldOrNew, index: number) => {
    if (oldOrNew === "old") {
        oldSelectedFileIndex.set(index)
    } else {
        newSelectedFileIndex.set(index)
    }
}

export const unselectFile = (oldOrNew: OldOrNew) => {
    if (oldOrNew === "old") {
        oldSelectedFileIndex.set(null)
    } else {
        newSelectedFileIndex.set(null)
    }
}

export const isSelected = (oldOrNew: OldOrNew, index: number): boolean => {
    const selectedIndex: number | null =
        (oldOrNew === "old") ?
            get(oldSelectedFileIndex)
            :
            get(newSelectedFileIndex)

    return selectedIndex === index
}

export const changeDir = async (oldOrNew: OldOrNew, dirname: string, currentDirPath?: string) => {
    if (get(changeDirProcessing)) return

    const _currentDirpath = currentDirPath !== undefined ? currentDirPath : currentDirpath(oldOrNew)
    if (_currentDirpath === null) return

    const dirpath = pathJoin(dirname, _currentDirpath)

    changeDirProcessing.set(true)
    const listDirResponse = await listDir(dirpath)
    changeDirProcessing.set(false)

    if (listDirResponse === null) {
        errorToast(T('Failed to change dir'))
        return
    }

    if (oldOrNew === 'old') {
        oldListDirResponse.set(listDirResponse)
        oldSelectedFileIndex.set(null)
    } else {
        newListDirResponse.set(listDirResponse)
        newSelectedFileIndex.set(null)
    }
}

export const syncDir = (oldOrNew: OldOrNew) => {
    if (oldOrNew === 'old') {
        newListDirResponse.set(get(oldListDirResponse))
    } else {
        oldListDirResponse.set(get(newListDirResponse))
    }
}

// todo: disable sync-dir button(s)
// export const currentDirIsEqual = (): boolean => {
//     return get(oldListDirResponse) !== null && get(newListDirResponse) !== null &&
//         get(oldListDirResponse)!.currentDir === get(newListDirResponse)!.currentDir
// }

export const pushCompareSetFromSelectedFiles = async () => {
    const oldFilepath: string | null = selectedFilepath("old")
    if (oldFilepath === null) return
    const newFilepath: string | null = selectedFilepath("new")
    if (newFilepath === null) return

    pushCompareSetFromFilepaths(oldFilepath, newFilepath)
}

// "ready" means either of:
// case: same name files exist
// case: both old/new files are selected
export const pushCompareSetIfReady = async (oldOrNew: OldOrNew, index: number) => {
    const _oldListDirResponse = listDirResponse("old")
    if (_oldListDirResponse === null) return
    const _newListDirResponse = listDirResponse("new")
    if (_newListDirResponse === null) return

    if (await pushCompareSetIfSameNameFileExists(_oldListDirResponse, _newListDirResponse, oldOrNew, index)) return
    pushCompareSetIfOldNewFilesSelected(_oldListDirResponse, _newListDirResponse)
}

const pushCompareSetIfOldNewFilesSelected = async (
    _oldListDirResponse: ListDirResponse,
    _newListDirResponse: ListDirResponse
): Promise<boolean> => {
    const _oldSelectedFileIndex = get(oldSelectedFileIndex)
    if (_oldSelectedFileIndex === null) return false
    const _newSelectedFileIndex = get(newSelectedFileIndex)
    if (_newSelectedFileIndex === null) return false

    const oldFilepath = `${_oldListDirResponse.currentDir}/${_oldListDirResponse.files[_oldSelectedFileIndex].name}`
    const newFilepath = `${_newListDirResponse.currentDir}/${_newListDirResponse.files[_newSelectedFileIndex].name}`

    await pushCompareSetFromFilepaths(oldFilepath, newFilepath)

    return true
}

const pushCompareSetIfSameNameFileExists = async (
    _oldListDirResponse: ListDirResponse,
    _newListDirResponse: ListDirResponse,
    oldOrNew: OldOrNew,
    index: number
): Promise<boolean> => {
    const filename = oldOrNew === "old" ?
        _oldListDirResponse.files[index].name :
        _newListDirResponse.files[index].name

    const found = oldOrNew === "old" ?
        _newListDirResponse.files.find((x) => x.name === filename)
        : _oldListDirResponse.files.find((x) => x.name === filename)

    if (!found) return false

    const oldFilepath = `${_oldListDirResponse.currentDir}/${filename}`
    const newFilepath = `${_newListDirResponse.currentDir}/${filename}`

    await pushCompareSetFromFilepaths(oldFilepath, newFilepath)

    return true
}

export const setDigestDiffs = () => {
    if (get(oldListDirResponse) === null || get(newListDirResponse) === null) return

    fileDigestDiffs.set([])
    dirDigestDiffs.set([])

    if (get(oldListDirResponse)!.currentDir === get(newListDirResponse)!.currentDir)
        return

    let filenames = get(oldListDirResponse)!.files
        .filter((x) => get(newListDirResponse)!.files.some((y) => y.name === x.name))
        .map((x) => x.name)
    filenames.forEach((x) => {
        invokeWithGuard('file_digest_diff', {
            filename: x,
            oldDir: get(oldListDirResponse)!.currentDir,
            newDir: get(newListDirResponse)!.currentDir,
        }).then((res: BackendCommandResult) => {
            if (res.isError) return
            fileDigestDiffs.update((updated) => {
                const pushed: DigestDiff = { name: x, equal: res.response as boolean }
                updated.push(pushed)
                return updated
            })
        })
    })

    let dirnames = get(oldListDirResponse)!.dirs.filter((x) =>
        get(newListDirResponse)!.dirs.some((y) => y === x)
    )
    dirnames.forEach((x) => {
        invokeWithGuard('dir_digest_diff', {
            dirname: x,
            oldDir: get(oldListDirResponse)!.currentDir,
            newDir: get(newListDirResponse)!.currentDir,
        }).then((res: BackendCommandResult) => {
            if (res.isError) return
            dirDigestDiffs.update((updated) => {
                const pushed: DigestDiff = { name: x, equal: res.response as boolean }
                updated.push(pushed)
                return updated
            })
        })
    })
}

export const statusIconName = (name: string, isDir: boolean): string | null => {
    const found = isDir ? get(dirDigestDiffs).find((x) => x.name === name) : get(fileDigestDiffs).find((x) => x.name === name)
    if (!found) return null
    return found.equal ? "Check" : "TriangleAlert"
}

const currentDirpath = (oldOrNew: OldOrNew): string | null => {
    return oldOrNew === 'old' ? oldCurrentDirpath() : newCurrentDirpath()
}

const listDir = async (currentDir: string): Promise<ListDirResponse | null> => {
    const res = await invokeWithGuard('list_dir', { currentDir })
    if (res.isError) {
        return null
    }
    return res.response as ListDirResponse
}

const oldCurrentDirpath = (): string | null => {
    return get(oldListDirResponse) !== null ? get(oldListDirResponse)!.currentDir : null
}

const newCurrentDirpath = (): string | null => {
    return get(newListDirResponse) !== null ? get(newListDirResponse)!.currentDir : null
}

const selectedFilepath = (oldOrNew: OldOrNew): string | null => {
    const listDirResponse = oldOrNew === "old" ? get(oldListDirResponse) : get(newListDirResponse)
    if (listDirResponse === null) return null
    const selectedFileIndex = oldOrNew === "old" ? get(oldSelectedFileIndex) : get(newSelectedFileIndex)
    if (selectedFileIndex === null) return null
    return `${listDirResponse.currentDir}/${listDirResponse.files[selectedFileIndex].name}`
}
