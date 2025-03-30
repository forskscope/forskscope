import { invokeWithGuard } from "./backend.svelte"

export const openWithFileManager = (dirpath: string) => {
    invokeWithGuard('open_with_file_manager', { dirpath })
}

export const pathJoin = (filename: string, dirpath: string): string => {
    return `${dirpath}/${filename}`
}
