import * as React from 'react'
import { cn } from '../../lib/utils'
import {
  Send,
  FolderTree,
  History,
  Beaker,
  Plug,
  Settings,
  type LucideIcon,
} from 'lucide-react'

export interface ActivityBarItem {
  id: string
  icon: LucideIcon
  label: string
  badge?: number
}

export interface ActivityBarProps {
  items?: ActivityBarItem[]
  activeId?: string | null
  onSelect?: (id: string) => void
  className?: string
}

const defaultItems: ActivityBarItem[] = [
  { id: 'collections', icon: FolderTree, label: 'Collections' },
  { id: 'history', icon: History, label: 'History' },
  { id: 'env', icon: Beaker, label: 'Environments' },
  { id: 'plugins', icon: Plug, label: 'Plugins' },
  { id: 'settings', icon: Settings, label: 'Settings' },
]

export const ActivityBar = React.forwardRef<HTMLDivElement, ActivityBarProps>(
  ({ items = defaultItems, activeId, onSelect, className }, ref) => {
    return (
      <div
        ref={ref}
        className={cn(
          'flex flex-col items-center gap-1 w-12 py-2 bg-gray-900 text-gray-400 border-r border-gray-700',
          className
        )}
        data-testid="activity-bar"
      >
        <div className="flex items-center justify-center h-8 w-8 mb-2">
          <Send className="h-5 w-5 text-blue-400" />
        </div>
        {items.map((item) => {
          const Icon = item.icon
          const isActive = activeId === item.id
          return (
            <button
              key={item.id}
              onClick={() => onSelect?.(item.id)}
              className={cn(
                'relative flex items-center justify-center h-8 w-8 rounded transition-colors',
                isActive
                  ? 'bg-gray-700 text-white'
                  : 'hover:text-gray-200 hover:bg-gray-800'
              )}
              title={item.label}
              data-testid={`activity-${item.id}`}
            >
              <Icon className="h-5 w-5" />
              {item.badge ? (
                <span className="absolute -top-0.5 -right-0.5 h-3.5 w-3.5 rounded-full bg-red-500 text-[8px] font-bold text-white flex items-center justify-center">
                  {item.badge}
                </span>
              ) : null}
            </button>
          )
        })}
      </div>
    )
  }
)

ActivityBar.displayName = 'ActivityBar'
