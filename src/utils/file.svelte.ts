import { PATH_SEPARATOR } from "../stores/file.svelte"

export const pathJoin = (filename: string, dirpath: string): string => {
    return `${dirpath}${PATH_SEPARATOR!}${filename}`
}
