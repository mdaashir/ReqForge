import { invoke } from '@tauri-apps/api/core'
import { useCallback, useEffect, useState } from 'react'

export interface CredentialMeta {
  account: string
  created_at_ms: number
}

export interface UseKeychainReturn {
  accounts: CredentialMeta[]
  loading: boolean
  error: string | null
  get: (account: string) => Promise<string | null>
  set: (account: string, value: string) => Promise<void>
  remove: (account: string) => Promise<void>
  refresh: () => Promise<void>
}

/**
 * Hook that wraps the OS keychain Tauri commands. Credentials are
 * stored per-workspace and persist across app launches (via the OS
 * credential store: macOS Keychain, Windows Credential Manager,
 * Linux Secret Service).
 *
 * The browser build (vite.browser.config.ts) returns an `invoke`
 * stub from `browser.ts` that doesn't know about these commands, so
 * `set`/`remove` are no-ops in the browser.
 */
export function useKeychain(workspaceRoot: string | null): UseKeychainReturn {
  const [accounts, setAccounts] = useState<CredentialMeta[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const refresh = useCallback(async () => {
    if (!workspaceRoot) return
    setLoading(true)
    setError(null)
    try {
      const list = await invoke<CredentialMeta[]>('keychain_list', {
        workspaceRoot,
      })
      setAccounts(list)
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      setLoading(false)
    }
  }, [workspaceRoot])

  useEffect(() => {
    refresh()
  }, [refresh])

  const get = useCallback(async (account: string) => {
    try {
      return await invoke<string | null>('keychain_get', { account })
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      return null
    }
  }, [])

  const set = useCallback(
    async (account: string, value: string) => {
      if (!workspaceRoot) return
      try {
        await invoke('keychain_set', { workspaceRoot, account, value })
        await refresh()
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err))
      }
    },
    [workspaceRoot, refresh]
  )

  const remove = useCallback(
    async (account: string) => {
      if (!workspaceRoot) return
      try {
        await invoke('keychain_delete', { account })
        await refresh()
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err))
      }
    },
    [workspaceRoot, refresh]
  )

  return { accounts, loading, error, get, set, remove, refresh }
}
