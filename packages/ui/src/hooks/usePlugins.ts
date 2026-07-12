import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'

export interface PluginInfo {
  id: string
  name: string
  version: string
  author?: string
  description?: string
  permissions: string[]
}

export interface UsePluginsReturn {
  plugins: PluginInfo[]
  loading: boolean
  error: string | null
  reload: () => Promise<void>
  enable: (id: string) => Promise<void>
  disable: (id: string) => Promise<void>
}

/**
 * Hook for managing ReqForge plugins.
 *
 * Plugins are .wasm files placed under `<workspace>/plugins/`. The Rust
 * host enumerates them on `load` and the UI can enable/disable them
 * individually. Hooks fire transparently on request/response.
 */
export function usePlugins(): UsePluginsReturn {
  const [plugins, setPlugins] = useState<PluginInfo[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const reload = async () => {
    setLoading(true)
    setError(null)
    try {
      const list = await invoke<PluginInfo[]>('list_plugins')
      setPlugins(list)
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      setLoading(false)
    }
  }

  const enable = async (id: string) => {
    try {
      await invoke('enable_plugin', { id })
      await reload()
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    }
  }

  const disable = async (id: string) => {
    try {
      await invoke('disable_plugin', { id })
      await reload()
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    }
  }

  useEffect(() => {
    reload()
  }, [])

  return { plugins, loading, error, reload, enable, disable }
}
