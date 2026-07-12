import { useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { useRequestStore } from '../stores/requestStore'
import type { ApiResponse, Request } from '../types'

interface SendResult {
  success: boolean
  response?: ApiResponse
  error?: string
}

/**
 * Hook for sending HTTP requests through the Tauri IPC bridge to the Rust backend.
 *
 * Manages loading state, errors, and response data via the requestStore.
 */
export function useRequest() {
  const {
    request,
    response,
    loading,
    error,
    setRequest,
    setResponse,
    setLoading,
    setError,
    reset,
    undo,
    redo,
    canUndo,
    canRedo,
  } = useRequestStore()

  const sendRequest = useCallback(async (): Promise<SendResult> => {
    if (!request.url) {
      const errMsg = 'URL is required'
      setError(errMsg)
      return { success: false, error: errMsg }
    }

    setLoading(true)
    setError(null)

    try {
      const result = await invoke<ApiResponse>('send_request', { request })
      setResponse(result)

      // Best-effort history recording (don't block on failure)
      try {
        await invoke('record_history', {
          entry: {
            id: crypto.randomUUID(),
            timestamp: Math.floor(Date.now() / 1000),
            method: request.method,
            url: request.url,
            status: result.status,
            statusText: result.statusText,
            durationMs: result.timing.totalMs,
            sizeBytes: result.size.total,
            request,
            response: result,
          },
        })
      } catch (historyErr) {
        console.warn('History recording failed:', historyErr)
      }

      return { success: true, response: result }
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err)
      setError(errorMessage)
      return { success: false, error: errorMessage }
    } finally {
      setLoading(false)
    }
  }, [request, setLoading, setError, setResponse])

  const updateRequest = useCallback(
    (updates: Partial<Request>) => {
      setRequest({ ...request, ...updates })
    },
    [request, setRequest]
  )

  return {
    request,
    response,
    loading,
    error,
    sendRequest,
    updateRequest,
    reset,
    setRequest,
    undo,
    redo,
    canUndo,
    canRedo,
  }
}
