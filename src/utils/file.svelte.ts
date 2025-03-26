import { invoke } from "@tauri-apps/api/core"
import { PATH_SEPARATOR } from "../consts"

export const pathJoin = (filename: string, dirpath: string): string => {
    return `${dirpath}${PATH_SEPARATOR}${filename}`
}
