import { useCallback } from 'react'
import { useCollectionStore, type Collection } from '../stores/collectionStore'

interface UseCollectionReturn {
  collections: Collection[]
  activeId: string | null
  loading: boolean
  error: string | null
  loadAll: () => Promise<void>
  select: (id: string | null) => void
  create: (name: string) => Promise<Collection | null>
  delete: (id: string) => Promise<void>
  rename: (id: string, name: string) => Promise<void>
}

export function useCollection(): UseCollectionReturn {
  const store = useCollectionStore()

  const loadAll = useCallback(async () => {
    await store.loadAll()
  }, [store.loadAll])

  const select = useCallback((id: string | null) => {
    store.select(id)
  }, [store.select])

  const create = useCallback(
    async (name: string) => {
      return await store.create(name)
    },
    [store.create]
  )

  const deleteColl = useCallback(
    async (id: string) => {
      await store.delete(id)
    },
    [store.delete]
  )

  const rename = useCallback(
    async (id: string, name: string) => {
      await store.rename(id, name)
    },
    [store.rename]
  )

  return {
    collections: store.collections,
    activeId: store.activeId,
    loading: store.loading,
    error: store.error,
    loadAll,
    select,
    create,
    delete: deleteColl,
    rename,
  }
}
