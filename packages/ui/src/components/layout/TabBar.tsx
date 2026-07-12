import * as React from 'react'
import { cn } from '../../lib/utils'
import { X } from 'lucide-react'

export interface Tab {
  id: string
  title: string
  type?: string
  dirty?: boolean
}

export interface TabBarProps {
  tabs: Tab[]
  activeId: string | null
  onSelect: (id: string) => void
  onClose: (id: string) => void
  onReorder?: (fromIndex: number, toIndex: number) => void
  className?: string
}

export const TabBar = React.forwardRef<HTMLDivElement, TabBarProps>(
  ({ tabs, activeId, onSelect, onClose, className }, ref) => {
    const [dragIdx, setDragIdx] = React.useState<number | null>(null)

    const typeColor: Record<string, string> = {
      request: 'border-t-blue-500',
      graphql: 'border-t-pink-500',
      websocket: 'border-t-green-500',
      grpc: 'border-t-purple-500',
    }

    if (tabs.length === 0) return null

    return (
      <div
        ref={ref}
        className={cn(
          'flex items-center overflow-x-auto bg-gray-100 dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700',
          className
        )}
        data-testid="tab-bar"
      >
        {tabs.map((tab, idx) => {
          const isActive = tab.id === activeId
          return (
            <div
              key={tab.id}
              draggable
              onDragStart={() => setDragIdx(idx)}
              onDragOver={(e) => {
                e.preventDefault()
                if (dragIdx !== null && dragIdx !== idx) {
                  onReorder?.(dragIdx, idx)
                  setDragIdx(idx)
                }
              }}
              onDragEnd={() => setDragIdx(null)}
              className={cn(
                'group flex items-center gap-1.5 px-3 py-2 text-sm cursor-pointer select-none',
                'border-r border-gray-200 dark:border-gray-700',
                'border-t-2 min-w-0 max-w-[200px]',
                typeColor[tab.type ?? ''] || 'border-t-transparent',
                isActive
                  ? 'bg-white dark:bg-gray-900 text-gray-900 dark:text-gray-100'
                  : 'bg-gray-100 dark:bg-gray-800 text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-200'
              )}
              onClick={() => onSelect(tab.id)}
              data-testid={`tab-${tab.id}`}
            >
              <span className="truncate flex-1">{tab.title}</span>
              {tab.dirty && (
                <span className="h-2 w-2 rounded-full bg-yellow-500 flex-shrink-0" />
              )}
              <button
                onClick={(e) => {
                  e.stopPropagation()
                  onClose(tab.id)
                }}
                className="opacity-0 group-hover:opacity-100 hover:bg-gray-200 dark:hover:bg-gray-700 rounded p-0.5 flex-shrink-0"
                data-testid={`tab-close-${tab.id}`}
              >
                <X className="h-3 w-3" />
              </button>
            </div>
          )
        })}
      </div>
    )
  }
)

TabBar.displayName = 'TabBar'
