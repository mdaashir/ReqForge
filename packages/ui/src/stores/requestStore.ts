import { create } from 'zustand'
import type { ApiResponse, Request } from '../types'

/// Maximum number of past states to keep in the undo history.
/// Older states are dropped when this is exceeded.
const MAX_HISTORY = 50;

interface RequestState {
  request: Request
  response: ApiResponse | null
  loading: boolean
  error: string | null
  /// Stack of past `request` values, oldest at index 0.
  past: Request[]
  /// Stack of future `request` values for redo.
  future: Request[]

  setRequest: (request: Request) => void
  updateRequest: (updates: Partial<Request>) => void
  setResponse: (response: ApiResponse | null) => void
  setLoading: (loading: boolean) => void
  setError: (error: string | null) => void
  reset: () => void
  /// Undo the last request mutation.
  undo: () => void
  /// Redo a previously undone mutation.
  redo: () => void
  /// Returns true if there is something to undo.
  canUndo: () => boolean
  /// Returns true if there is something to redo.
  canRedo: () => boolean
}

function createDefaultRequest(): Request {
  return {
    id: crypto.randomUUID(),
    name: 'Untitled Request',
    method: 'GET',
    url: '',
    headers: [],
    params: [],
    body: { mode: 'none', content: '' },
    followRedirects: true,
    verifySSL: true,
  }
}

export const useRequestStore = create<RequestState>((set, get) => {
  /**
   * Push the current `request` onto the `past` stack and clear `future`.
   * Used by every mutation that should be undoable.
   */
  const pushHistory = () => {
    const state = get()
    const past = [...state.past, state.request]
    // Drop oldest entries past the limit.
    if (past.length > MAX_HISTORY) past.shift()
    set({ past, future: [] })
  }

  return {
    request: createDefaultRequest(),
    response: null,
    loading: false,
    error: null,
    past: [],
    future: [],

    setRequest: (request) => {
      pushHistory()
      set({ request })
    },

    updateRequest: (updates) => {
      pushHistory()
      set((state) => ({ request: { ...state.request, ...updates } }))
    },

    setResponse: (response) => set({ response }),

    setLoading: (loading) => set({ loading }),

    setError: (error) => set({ error }),

    reset: () => {
      pushHistory()
      set({
        request: createDefaultRequest(),
        response: null,
        loading: false,
        error: null,
      })
    },

    undo: () => {
      const state = get()
      if (state.past.length === 0) return
      const previous = state.past[state.past.length - 1]!
      const past = state.past.slice(0, -1)
      const future = [...state.future, state.request]
      set({ request: previous, past, future })
    },

    redo: () => {
      const state = get()
      if (state.future.length === 0) return
      const next = state.future[state.future.length - 1]!
      const future = state.future.slice(0, -1)
      const past = [...state.past, state.request]
      set({ request: next, past, future })
    },

    canUndo: () => get().past.length > 0,
    canRedo: () => get().future.length > 0,
  }
})
