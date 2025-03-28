import { invoke } from "@tauri-apps/api/core"

export const binaryComparisonOnly = async (filepath: string): Promise<boolean> => {
    const res = (await invoke('binary_comparison_only', { filepath })
        // todo
        .catch((error: unknown) => {
            console.error(error)
            return
        }))
    return res as unknown as boolean
}
