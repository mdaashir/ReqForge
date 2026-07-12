import { useEffect, useState, useCallback } from 'react'

export interface CursorPosition {
  request_id: string
  field: string
  offset: number
}

export interface AwarenessState {
  user_name?: string | null
  user_color?: string | null
  focused_request?: string | null
  cursor?: CursorPosition | null
}

export interface Peer {
  client_id: number
  state: AwarenessState
  last_seen_ms: number
}

export interface AwarenessSnapshot {
  peers: Peer[]
}

export interface UseAwarenessReturn {
  peers: Peer[]
  local: AwarenessState
  setLocal: (state: Partial<AwarenessState>) => void
}

/**
 * Tracks presence + cursor positions of remote collaborators. The actual
 * transport (WebSocket, WebRTC, in-memory pub/sub) is supplied by the
 * host application via `subscribe`/`publish` props. This hook handles the
 * merge + lifecycle so multiple UI surfaces stay in sync.
 */
export interface UseAwarenessOptions {
  /** Subscribe to remote awareness updates. Returns an unsubscribe fn. */
  subscribe?: (cb: (snapshot: AwarenessSnapshot) => void) => () => void
  /** Publish a local awareness delta. Called whenever local state changes. */
  publish?: (state: AwarenessState) => void
  /** Initial local state. */
  initial?: AwarenessState
  /** Stale-after timeout in milliseconds (default 15000). */
  staleAfterMs?: number
}

export function useAwareness(options: UseAwarenessOptions = {}): UseAwarenessReturn {
  const { subscribe, publish, initial, staleAfterMs = 15_000 } = options
  const [local, setLocalState] = useState<AwarenessState>(initial ?? defaultLocal())
  const [peers, setPeers] = useState<Peer[]>([])

  // Subscribe to remote updates
  useEffect(() => {
    if (!subscribe) return
    return subscribe((snapshot) => {
      const now = Date.now()
      setPeers(snapshot.peers.filter((p) => now - p.last_seen_ms < staleAfterMs))
    })
  }, [subscribe, staleAfterMs])

  // Publish local changes
  useEffect(() => {
    publish?.(local)
  }, [local, publish])

  const setLocal = useCallback((delta: Partial<AwarenessState>) => {
    setLocalState((prev) => ({ ...prev, ...delta }))
  }, [])

  return { peers, local, setLocal }
}

function defaultLocal(): AwarenessState {
  return {
    user_name: generateName(),
    user_color: generateColor(),
    focused_request: null,
    cursor: null,
  }
}

const NAMES = [
  'Ada', 'Grace', 'Alan', 'Linus', 'Margaret', 'Donald', 'Barbara', 'Ken', 'Edsger', 'Brian',
]
const COLORS = [
  '#ef4444', '#f59e0b', '#10b981', '#3b82f6', '#8b5cf6', '#ec4899', '#06b6d4', '#84cc16',
]

function generateName(): string {
  return NAMES[Math.floor(Math.random() * NAMES.length)]!
}

function generateColor(): string {
  return COLORS[Math.floor(Math.random() * COLORS.length)]!
}
