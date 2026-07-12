import { useState } from 'react'
import { invoke } from '@tauri-apps/api/core'

export interface OAuthFlowRequest {
  authorization_endpoint: string
  token_endpoint: string
  client_id: string
  scopes: string[]
  extra_auth_params: [string, string][]
  extra_token_params: [string, string][]
}

export interface OAuthFlowResult {
  access_token: string
  refresh_token?: string
  expires_in?: number
  token_type: string
  scope?: string
}

export interface UseOAuthFlowReturn {
  running: boolean
  error: string | null
  startFlow: (req: OAuthFlowRequest) => Promise<OAuthFlowResult | null>
}

/**
 * Hook that triggers the OAuth 2.0 PKCE flow on the Rust backend.
 * The backend opens the system browser, starts a local server to
 * catch the redirect, exchanges the code for tokens, and returns
 * the result.
 */
export function useOAuthFlow(): UseOAuthFlowReturn {
  const [running, setRunning] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const startFlow = async (req: OAuthFlowRequest): Promise<OAuthFlowResult | null> => {
    setRunning(true)
    setError(null)
    try {
      const result = await invoke<OAuthFlowResult>('start_oauth_flow', { req })
      return result
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      return null
    } finally {
      setRunning(false)
    }
  }

  return { running, error, startFlow }
}
