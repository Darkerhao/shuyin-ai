import { invoke } from '@tauri-apps/api/core'
export interface InterfacePreferences { sidebarPrompt: boolean }
export const interfacePreferences = () => invoke<InterfacePreferences>('interface_preferences')
export const setSidebarPrompt = (visible: boolean) => invoke<void>('set_sidebar_prompt', { visible })
