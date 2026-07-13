import { create } from 'zustand'

export interface Tab {
  id: string
  title: string
  type: 'request' | 'response' | 'graphql' | 'websocket' | 'grpc'
  requestId?: string
}

interface TabState {
  tabs: Tab[]
  activeId: string | null

  open: (tab: Tab) => void
  close: (id: string) => void
  closeOthers: (id: string) => void
  closeAll: () => void
  activate: (id: string) => void
  reorder: (fromIndex: number, toIndex: number) => void
  rename: (id: string, title: string) => void
}

export const useTabStore = create<TabState>((set, get) => ({
  tabs: [],
  activeId: null,

  open: (tab) => {
    const existing = get().tabs.find((t) => t.id === tab.id)
    if (existing) {
      set({ activeId: tab.id })
      return
    }
    set((state) => ({
      tabs: [...state.tabs, tab],
      activeId: tab.id,
    }))
  },

  close: (id) => {
    set((state) => {
      const idx = state.tabs.findIndex((t) => t.id === id)
      const tabs = state.tabs.filter((t) => t.id !== id)
      let activeId = state.activeId
      if (state.activeId === id) {
        // Switch to the nearest neighbour
        if (tabs.length === 0) {
          activeId = null
        } else if (idx > 0) {
          activeId = tabs[Math.min(idx - 1, tabs.length - 1)]!.id
        } else {
          activeId = tabs[0]!.id
        }
      }
      return { tabs, activeId }
    })
  },

  closeOthers: (id) => {
    set((state) => ({
      tabs: state.tabs.filter((t) => t.id === id),
      activeId: id,
    }))
  },

  closeAll: () => set({ tabs: [], activeId: null }),

  activate: (id) => set({ activeId: id }),

  reorder: (fromIndex, toIndex) => {
    set((state) => {
      const tabs = [...state.tabs]
      const [removed] = tabs.splice(fromIndex, 1)
      tabs.splice(toIndex, 0, removed!)
      return { tabs }
    })
  },

  rename: (id, title) => {
    set((state) => ({
      tabs: state.tabs.map((t) => (t.id === id ? { ...t, title } : t)),
    }))
  },
}))
