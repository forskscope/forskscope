import { PATH_SEPARATOR } from "../../stores/file.svelte"
import type { ListDirResponse } from "../../types/file"
import { invokeWithGuard } from "../../utils/backend.svelte"

export const listDir = async (currentDir: string): Promise<ListDirResponse | null> => {
    const res = await invokeWithGuard('list_dir', { currentDir })
    if (res.isError) return null
    return res.response as ListDirResponse
}

export const lastSlashIndex = (dirpath: string): number => {
    return dirpath.lastIndexOf("/")
}

export const parentDirsPath = (dirpath: string): string => {
    return dirpath.substring(0, lastSlashIndex(dirpath) + 1)
}

export const extractDirname = (dirpath: string): string => {
    return dirpath.substring(lastSlashIndex(dirpath) + 1)
}

export const isRootDir = (dir: string): boolean => {
    // todo: frontend path separator should be integrated to `/` ?
    if (PATH_SEPARATOR! === '\\') {
        return dir.endsWith(`:${PATH_SEPARATOR!}`)
    } else {
        return dir === PATH_SEPARATOR!
    }
}
