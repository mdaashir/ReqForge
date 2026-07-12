// Browser stub for @tauri-apps/api/core
// In a PWA build, `invoke` calls are no-ops or fall back to localStorage.

export async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  console.debug('[browser] invoke', cmd, args)

  // Hardcoded responses for commands the UI calls on startup.
  if (cmd === 'list_collections')   return [] as unknown as T
  if (cmd === 'list_environments')  return [] as unknown as T
  if (cmd === 'ping')               return { pong: true } as unknown as T
  if (cmd === 'get_app_version')    return '0.1.0-dev' as unknown as T
  if (cmd === 'get_app_name')       return 'ReqForge (Browser)' as unknown as T
  if (cmd === 'init_workspace')     return undefined as unknown as T

  // Load data from localStorage if available.
  const key = `reqforge:${cmd}:${JSON.stringify(args ?? {})}`
  const stored = localStorage.getItem(key)
  if (stored) {
    try { return JSON.parse(stored) as T } catch { /* ignore */ }
  }

  // Default: return empty/undefined for anything we don't handle.
  return undefined as unknown as T
}

/** Persist data to localStorage for browser demo sessions. */
export async function store<T>(cmd: string, args: Record<string, unknown>, data: T): Promise<void> {
  const key = `reqforge:${cmd}:${JSON.stringify(args)}`
  localStorage.setItem(key, JSON.stringify(data))
}
