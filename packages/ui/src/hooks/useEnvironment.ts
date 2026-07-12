import { invoke } from '@tauri-apps/api/core'
import type { Environment } from '../types'

export interface UseEnvironmentReturn {
  save: (env: Environment) => Promise<void>
  load: (name: string) => Promise<Environment>
  list: () => Promise<string[]>
  deleteEnv: (name: string) => Promise<void>
}

/**
 * Hook for environment persistence operations.
 *
 * Provides methods to save, load, list, and delete environments from disk.
 */
export function useEnvironment(): UseEnvironmentReturn {
  const save = async (env: Environment): Promise<void> => {
    await invoke('save_environment', { env })
  }

  const load = async (name: string): Promise<Environment> => {
    return await invoke('load_environment', { name })
  }

  const list = async (): Promise<string[]> => {
    return await invoke('list_environments')
  }

  const deleteEnv = async (name: string): Promise<void> => {
    await invoke('delete_environment', { name })
  }

  return { save, load, list, deleteEnv }
}
