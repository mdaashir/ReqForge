import { create } from 'zustand'
import { invoke } from '@tauri-apps/api/core'

export interface HistoryItem {
  id: string
  timestamp: number // unix seconds
  method: string
  url: string
  status: number
  statusText: string
  durationMs: number
  sizeBytes: number
}

interface HistoryState {
  items: HistoryItem[]
  loading: boolean
  searchQuery: string

  load: () => Promise<void>
  search: (query: string) => Promise<void>
  clear: () => Promise<void>
  record: (entry: HistoryItem) => Promise<void>
  setSearchQuery: (q: string) => void
  replay: (id: string) => Promise<void>
}

/**
 * History store backed by the Rust `reqforge_core::history::HistoryStorage`.
 * The frontend only keeps a flattened view; full request/response lives in Rust.
 */
export const useHistoryStore = create<HistoryState>((set, get) => ({
  items: [],
  loading: false,
  searchQuery: '',

  load: async () => {
    set({ loading: true })
    try {
      const items = await invoke<HistoryItem[]>('list_history', { limit: 200 })
      set({ items, loading: false })
    } catch (err) {
      console.error('Failed to load history:', err)
      set({ loading: false })
    }
  },

  search: async (query: string) => {
    if (!query.trim()) {
      await get().load()
      return
    }
    set({ loading: true })
    try {
      const items = await invoke<HistoryItem[]>('search_history', {
        needle: query,
        limit: 100,
      })
      set({ items, loading: false })
    } catch (err) {
      console.error('Failed to search history:', err)
      set({ loading: false })
    }
  },

  clear: async () => {
    try {
      await invoke('clear_history')
      set({ items: [] })
    } catch (err) {
      console.error('Failed to clear history:', err)
    }
  },

  record: async (entry: HistoryItem) => {
    try {
      // Full record (with request/response) is sent from Rust via record_history.
      // This entry shape is for display only.
      set((state) => ({ items: [entry, ...state.items].slice(0, 200) }))
    } catch (err) {
      console.error('Failed to record history:', err)
    }
  },

  setSearchQuery: (q) => set({ searchQuery: q }),

  /**
   * Re-send a previously executed request. The Rust side looks up the full
   * request payload from history and returns it; the caller is expected to
   * feed it back into `useRequest` to actually fire it.
   */
  replay: async (id: string): Promise<void> => {
    try {
      await invoke('replay_history', { id })
    } catch (err) {
      console.error('Failed to replay history item:', err)
    }
  },
}))
