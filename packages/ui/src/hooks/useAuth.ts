import { useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'

export type { AuthConfig } from '../types'

export type AuthType = 'none' | 'apiKey' | 'bearer' | 'basic' | 'oauth2' | 'jwt' | 'awsSigV4'

interface UseAuthReturn {
  getToken: (provider: string, config: Record<string, string>) => Promise<string | null>
  refreshToken: (provider: string, config: Record<string, string>) => Promise<string | null>
  storeCredential: (key: string, value: string) => Promise<void>
  getCredential: (key: string) => Promise<string | null>
  deleteCredential: (key: string) => Promise<void>
  listCredentials: () => Promise<string[]>
}

export function useAuth(): UseAuthReturn {
  const getToken = useCallback(
    async (provider: string, config: Record<string, string>): Promise<string | null> => {
      try {
        return await invoke<string>('get_auth_token', { provider, config })
      } catch (err) {
        console.error('Failed to get token:', err)
        return null
      }
    },
    []
  )

  const refreshToken = useCallback(
    async (provider: string, config: Record<string, string>): Promise<string | null> => {
      try {
        return await invoke<string>('refresh_auth_token', { provider, config })
      } catch (err) {
        console.error('Failed to refresh token:', err)
        return null
      }
    },
    []
  )

  const storeCredential = useCallback(
    async (key: string, value: string): Promise<void> => {
      try {
        await invoke('store_credential', { key, value })
      } catch (err) {
        console.error('Failed to store credential:', err)
      }
    },
    []
  )

  const getCredential = useCallback(
    async (key: string): Promise<string | null> => {
      try {
        return await invoke<string | null>('get_credential', { key })
      } catch (err) {
        console.error('Failed to get credential:', err)
        return null
      }
    },
    []
  )

  const deleteCredential = useCallback(
    async (key: string): Promise<void> => {
      try {
        await invoke('delete_credential', { key })
      } catch (err) {
        console.error('Failed to delete credential:', err)
      }
    },
    []
  )

  const listCredentials = useCallback(async (): Promise<string[]> => {
    try {
      return await invoke<string[]>('list_credentials')
    } catch (err) {
      console.error('Failed to list credentials:', err)
      return []
    }
  }, [])

  return {
    getToken,
    refreshToken,
    storeCredential,
    getCredential,
    deleteCredential,
    listCredentials,
  }
}
