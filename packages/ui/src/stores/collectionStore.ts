import { create } from 'zustand'
import { invoke } from '@tauri-apps/api/core'

export interface CollectionItemRef {
  id: string
  name: string
  type: 'request' | 'folder'
  children?: CollectionItemRef[]
}

export interface Collection {
  id: string
  name: string
  description?: string
  items: CollectionItemRef[]
}

interface CollectionState {
  collections: Collection[]
  activeId: string | null
  loading: boolean
  error: string | null

  loadAll: () => Promise<void>
  select: (id: string | null) => void
  create: (name: string) => Promise<Collection | null>
  delete: (id: string) => Promise<void>
  rename: (id: string, name: string) => Promise<void>
  move: (id: string, targetId: string, position: number) => Promise<void>
}

export const useCollectionStore = create<CollectionState>((set, get) => ({
  collections: [],
  activeId: null,
  loading: false,
  error: null,

  loadAll: async () => {
    set({ loading: true, error: null })
    try {
      const collections = await invoke<Collection[]>('list_collections')
      set({ collections, loading: false })
    } catch (err) {
      console.error('Failed to load collections:', err)
      set({ loading: false, error: String(err) })
    }
  },

  select: (id) => set({ activeId: id }),

  create: async (name) => {
    try {
      const created = await invoke<Collection>('create_collection', { name })
      set((state) => ({ collections: [...state.collections, created] }))
      return created
    } catch (err) {
      console.error('Failed to create collection:', err)
      set({ error: String(err) })
      return null
    }
  },

  delete: async (id) => {
    try {
      await invoke('delete_collection', { id })
      set((state) => ({
        collections: state.collections.filter((c) => c.id !== id),
        activeId: state.activeId === id ? null : state.activeId,
      }))
    } catch (err) {
      console.error('Failed to delete collection:', err)
      set({ error: String(err) })
    }
  },

  rename: async (id, name) => {
    try {
      await invoke('rename_collection', { id, name })
      set((state) => ({
        collections: state.collections.map((c) =>
          c.id === id ? { ...c, name } : c
        ),
      }))
    } catch (err) {
      console.error('Failed to rename collection:', err)
      set({ error: String(err) })
    }
  },

  move: async (id, targetId, position) => {
    try {
      await invoke('move_collection_item', { id, targetId, position })
      await get().loadAll()
    } catch (err) {
      console.error('Failed to move collection item:', err)
      set({ error: String(err) })
    }
  },
}))
