import * as React from 'react'
import { cn } from '../../lib/utils'

export interface HeadersViewerProps {
  headers: Record<string, string>
  className?: string
}

export const HeadersViewer = React.forwardRef<HTMLDivElement, HeadersViewerProps>(
  ({ headers, className }, ref) => {
    const entries = Object.entries(headers)
    if (entries.length === 0) {
      return (
        <div
          ref={ref}
          className={cn('p-3 text-xs text-gray-400 italic', className)}
          data-testid="headers-viewer"
        >
          No headers
        </div>
      )
    }

    return (
      <div
        ref={ref}
        className={cn(
          'p-3 rounded-md bg-gray-50 dark:bg-gray-900 border border-gray-200 dark:border-gray-700 overflow-auto max-h-96',
          className
        )}
        data-testid="headers-viewer"
      >
        <table className="w-full text-xs">
          <tbody>
            {entries.map(([key, value]) => (
              <tr key={key} className="border-b border-gray-100 dark:border-gray-800">
                <td className="py-1 pr-3 font-mono font-semibold text-gray-700 dark:text-gray-300 whitespace-nowrap">
                  {key}
                </td>
                <td className="py-1 font-mono text-gray-600 dark:text-gray-400 break-all">
                  {value}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    )
  }
)

HeadersViewer.displayName = 'HeadersViewer'
