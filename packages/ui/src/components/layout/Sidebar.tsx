import * as React from 'react'
import { cn } from '../../lib/utils'

export interface SidebarProps {
  children: React.ReactNode
  width?: number
  minWidth?: number
  collapsible?: boolean
  defaultCollapsed?: boolean
  className?: string
  onResize?: (width: number) => void
}

export const Sidebar = React.forwardRef<HTMLDivElement, SidebarProps>(
  (
    {
      children,
      width: initialWidth = 280,
      minWidth = 200,
      collapsible = true,
      defaultCollapsed = false,
      className,
      onResize,
    },
    ref
  ) => {
    const [width, setWidth] = React.useState(initialWidth)
    const [collapsed, setCollapsed] = React.useState(defaultCollapsed)
    const [resizing, setResizing] = React.useState(false)

    const handleMouseDown = (e: React.MouseEvent) => {
      e.preventDefault()
      setResizing(true)
    }

    React.useEffect(() => {
      if (!resizing) return
      const handleMouseMove = (e: MouseEvent) => {
        const newWidth = Math.max(minWidth, e.clientX)
        setWidth(newWidth)
        onResize?.(newWidth)
      }
      const handleMouseUp = () => setResizing(false)
      document.addEventListener('mousemove', handleMouseMove)
      document.addEventListener('mouseup', handleMouseUp)
      return () => {
        document.removeEventListener('mousemove', handleMouseMove)
        document.removeEventListener('mouseup', handleMouseUp)
      }
    }, [resizing, minWidth, onResize])

    return (
      <div
        ref={ref}
        className={cn(
          'relative flex flex-col bg-gray-50 dark:bg-gray-900 border-r border-gray-200 dark:border-gray-700',
          'overflow-hidden transition-all duration-200',
          collapsed ? 'w-0 min-w-0 border-r-0' : 'flex-shrink-0',
          className
        )}
        style={collapsed ? {} : { width }}
        data-testid="sidebar"
      >
        <div className="flex-1 overflow-auto">
          {children}
        </div>
        {collapsible && (
          <button
            onClick={() => setCollapsed(!collapsed)}
            className={cn(
              'absolute right-0 top-1/2 -translate-y-1/2 translate-x-1/2 z-10',
              'flex items-center justify-center h-8 w-5 rounded-r bg-gray-200 dark:bg-gray-700',
              'hover:bg-gray-300 dark:hover:bg-gray-600 text-gray-500',
              'border border-gray-300 dark:border-gray-600 border-l-0'
            )}
            title={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
            data-testid="sidebar-toggle"
          >
            <svg className={cn('h-3 w-3 transition-transform', collapsed ? 'rotate-180' : '')} viewBox="0 0 16 16" fill="currentColor">
              <path d="M10 12L6 8l4-4" />
            </svg>
          </button>
        )}
        {resizing && (
          <div
            className="fixed inset-0 z-50 cursor-col-resize"
            data-testid="sidebar-resize-overlay"
          />
        )}
        {/* Resize handle */}
        <div
          onMouseDown={handleMouseDown}
          className="absolute right-0 top-0 bottom-0 w-1 cursor-col-resize hover:bg-blue-500 hover:opacity-50 z-10"
          data-testid="sidebar-resize-handle"
        />
      </div>
    )
  }
)

Sidebar.displayName = 'Sidebar'
