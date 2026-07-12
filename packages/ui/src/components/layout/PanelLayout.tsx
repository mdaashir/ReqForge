import * as React from 'react'
import { cn } from '../../lib/utils'

export interface PanelLayoutProps {
  children: React.ReactNode
  /** Optional bottom panel */
  bottom?: React.ReactNode
  defaultBottomHeight?: number
  minBottomHeight?: number
  className?: string
}

export const PanelLayout = React.forwardRef<HTMLDivElement, PanelLayoutProps>(
  ({ children, bottom, defaultBottomHeight = 200, minBottomHeight = 100, className }, ref) => {
    const [height, setHeight] = React.useState(defaultBottomHeight)
    const [resizing, setResizing] = React.useState(false)
    const [bottomOpen, setBottomOpen] = React.useState(!!bottom)

    const handleMouseDown = (e: React.MouseEvent) => {
      e.preventDefault()
      setResizing(true)
    }

    React.useEffect(() => {
      if (!resizing) return
      const handleMouseMove = (e: MouseEvent) => {
        const newHeight = Math.max(minBottomHeight, window.innerHeight - e.clientY)
        setHeight(newHeight)
      }
      const handleMouseUp = () => setResizing(false)
      document.addEventListener('mousemove', handleMouseMove)
      document.addEventListener('mouseup', handleMouseUp)
      return () => {
        document.removeEventListener('mousemove', handleMouseMove)
        document.removeEventListener('mouseup', handleMouseUp)
      }
    }, [resizing, minBottomHeight])

    return (
      <div
        ref={ref}
        className={cn('flex flex-col flex-1 overflow-hidden', className)}
        data-testid="panel-layout"
      >
        <div className="flex-1 overflow-auto">
          {children}
        </div>
        {bottom && bottomOpen && (
          <>
            <div
              onMouseDown={handleMouseDown}
              className="h-1 cursor-row-resize hover:bg-blue-500 hover:opacity-50 flex-shrink-0 z-10"
              data-testid="panel-resize-handle"
            />
            <div
              style={{ height }}
              className="flex-shrink-0 overflow-auto border-t border-gray-200 dark:border-gray-700"
              data-testid="bottom-panel"
            >
              {bottom}
            </div>
          </>
        )}
        {bottom && !bottomOpen && (
          <button
            onClick={() => setBottomOpen(true)}
            className="flex items-center justify-center h-6 text-xs text-gray-500 hover:text-gray-700 dark:hover:text-gray-300 border-t border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800"
            data-testid="panel-bottom-toggle"
          >
            Show panel
          </button>
        )}
        {resizing && (
          <div
            className="fixed inset-0 z-50 cursor-row-resize"
            data-testid="panel-resize-overlay"
          />
        )}
      </div>
    )
  }
)

PanelLayout.displayName = 'PanelLayout'
