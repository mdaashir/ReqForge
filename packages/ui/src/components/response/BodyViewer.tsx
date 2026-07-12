import * as React from 'react'
import { cn } from '../../lib/utils'

export interface BodyViewerProps {
  body: string
  contentType?: string
  className?: string
}

export function formatBody(body: string, contentType?: string): string {
  if (!body) return ''
  if (contentType?.includes('json')) {
    try {
      return JSON.stringify(JSON.parse(body), null, 2)
    } catch {
      return body
    }
  }
  return body
}

export const BodyViewer = React.forwardRef<HTMLPreElement, BodyViewerProps>(
  ({ body, contentType, className }, ref) => {
    return (
      <pre
        ref={ref}
        className={cn(
          'p-3 rounded-md bg-gray-50 dark:bg-gray-900 border border-gray-200 dark:border-gray-700 overflow-auto text-xs font-mono max-h-96',
          className
        )}
        data-testid="body-viewer"
      >
        {formatBody(body, contentType)}
      </pre>
    )
  }
)

BodyViewer.displayName = 'BodyViewer'
