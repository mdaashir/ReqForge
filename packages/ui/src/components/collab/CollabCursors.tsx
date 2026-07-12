import * as React from 'react'
import { cn } from '../../lib/utils'
import type { Peer, CursorPosition } from '../../hooks/useAwareness'

export interface CollabCursorsProps {
  peers: Peer[]
  /** Map of request_id → DOM ref. Cursors are positioned relative to these. */
  refs: Record<string, React.RefObject<HTMLElement | null>>
  className?: string
}

/**
 * Renders floating cursor labels for every remote peer. Each cursor shows
 * a colored caret + a name pill. Position is computed from the peer's
 * `cursor.offset` against the request element's text content.
 */
export const CollabCursors: React.FC<CollabCursorsProps> = ({ peers, refs, className }) => {
  if (peers.length === 0) return null

  return (
    <div
      className={cn('pointer-events-none fixed inset-0 z-40', className)}
      aria-hidden="true"
      data-testid="collab-cursors"
    >
      {peers.map((peer) => {
        const cursor = peer.state.cursor
        if (!cursor) return null
        const ref = refs[cursor.request_id]
        const target = ref?.current
        if (!target) return null
        const pos = projectOffset(target, cursor)
        if (!pos) return null
        return (
          <div
            key={peer.client_id}
            className="absolute transition-all duration-150 ease-out"
            style={{
              transform: `translate(${pos.x}px, ${pos.y}px)`,
            }}
            data-testid={`cursor-${peer.client_id}`}
          >
            <svg width="20" height="22" viewBox="0 0 20 22" fill="none">
              <path
                d="M2 2L18 10L10 11L7 18L2 2Z"
                fill={peer.state.user_color ?? '#3b82f6'}
                stroke="white"
                strokeWidth="1.5"
              />
            </svg>
            <span
              className="absolute left-4 top-4 px-1.5 py-0.5 rounded text-[10px] font-medium text-white whitespace-nowrap shadow"
              style={{ backgroundColor: peer.state.user_color ?? '#3b82f6' }}
            >
              {peer.state.user_name ?? 'Anonymous'}
            </span>
          </div>
        )
      })}
    </div>
  )
}

/**
 * Very simple cursor projection: measure the offset of the Nth text node
 * within the target element using a hidden Range. Returns null if the
 * offset exceeds the available text length (e.g. if the request element
 * has shrunk since the peer sent the cursor).
 */
function projectOffset(
  target: HTMLElement,
  cursor: CursorPosition
): { x: number; y: number } | null {
  if (cursor.field !== 'url') {
    // Only URL-bar cursors are supported in this MVP. Other fields can
    // fall back to a "top-left" anchor inside the request card.
    const rect = target.getBoundingClientRect()
    return { x: rect.left + 8, y: rect.top + 8 }
  }
  const urlEl = target.querySelector<HTMLElement>('[data-collab-field="url"]')
  if (!urlEl) {
    const rect = target.getBoundingClientRect()
    return { x: rect.left + 8, y: rect.top + 8 }
  }
  const range = document.createRange()
  const textNode = urlEl.firstChild
  if (!textNode || textNode.nodeType !== Node.TEXT_NODE) {
    const rect = urlEl.getBoundingClientRect()
    return { x: rect.left + 4, y: rect.top }
  }
  const offset = Math.min(cursor.offset, textNode.textContent?.length ?? 0)
  try {
    range.setStart(textNode, offset)
    range.setEnd(textNode, offset)
  } catch {
    return null
  }
  const rect = range.getBoundingClientRect()
  return { x: rect.left, y: rect.top }
}

/**
 * Helper component that wraps the avatar pills shown in the top bar.
 * Renders one chip per remote peer with their colour + initials.
 */
export const PresenceAvatars: React.FC<{ peers: Peer[]; className?: string }> = ({
  peers,
  className,
}) => {
  if (peers.length === 0) return null
  return (
    <div
      className={cn('flex items-center -space-x-2', className)}
      data-testid="presence-avatars"
    >
      {peers.slice(0, 5).map((peer) => (
        <div
          key={peer.client_id}
          title={peer.state.user_name ?? 'Anonymous'}
          className="h-7 w-7 rounded-full ring-2 ring-white dark:ring-gray-900 flex items-center justify-center text-xs font-semibold text-white shadow"
          style={{ backgroundColor: peer.state.user_color ?? '#3b82f6' }}
        >
          {(peer.state.user_name ?? '?').slice(0, 1).toUpperCase()}
        </div>
      ))}
      {peers.length > 5 && (
        <div className="h-7 w-7 rounded-full ring-2 ring-white dark:ring-gray-900 flex items-center justify-center text-xs font-semibold text-gray-700 dark:text-gray-300 bg-gray-200 dark:bg-gray-700">
          +{peers.length - 5}
        </div>
      )}
    </div>
  )
}
