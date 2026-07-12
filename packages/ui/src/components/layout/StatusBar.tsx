import * as React from 'react'
import { cn } from '../../lib/utils'

export interface StatusBarItem {
  id: string
  label: string
  icon?: React.ReactNode
  onClick?: () => void
}

export interface StatusBarProps {
  items?: StatusBarItem[]
  leftText?: string
  rightText?: string
  className?: string
}

export const StatusBar = React.forwardRef<HTMLDivElement, StatusBarProps>(
  ({ items, leftText, rightText, className }, ref) => {
    return (
      <div
        ref={ref}
        className={cn(
          'flex items-center justify-between h-6 px-3 text-xs',
          'bg-blue-900 text-blue-100 dark:bg-gray-900 dark:text-gray-400',
          className
        )}
        data-testid="status-bar"
      >
        <div className="flex items-center gap-3">
          {leftText && <span>{leftText}</span>}
          {items?.map((item) => (
            <button
              key={item.id}
              onClick={item.onClick}
              className="flex items-center gap-1 hover:text-white transition-colors"
              data-testid={`status-item-${item.id}`}
            >
              {item.icon}
              <span>{item.label}</span>
            </button>
          ))}
        </div>
        {rightText && (
          <div className="text-blue-300 dark:text-gray-500">
            <span>{rightText}</span>
          </div>
        )}
      </div>
    )
  }
)

StatusBar.displayName = 'StatusBar'
