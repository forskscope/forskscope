import { open as tauriDialogOpen } from '@tauri-apps/plugin-dialog'

export const filepathFromDialog = async (): Promise<string | null> => {
    const filepath: string | null = await tauriDialogOpen({
        filters: [
            {
                name: 'All files',
                extensions: ['*'],
            },
        ],
    })
    return filepath
}
