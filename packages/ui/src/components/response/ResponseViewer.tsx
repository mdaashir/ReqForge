import * as React from 'react'
import { Clock, FileText, Hash, Zap, GanttChart } from 'lucide-react'
import { Tabs, TabsList, TabsTrigger, TabsContent } from '../primitives/Tabs'
import { cn, formatBytes, formatDuration } from '../../lib/utils'
import { BodyViewer } from './BodyViewer'
import { HeadersViewer } from './HeadersViewer'
import { TimingViewer } from './TimingViewer'
import type { ApiResponse } from '../../types'

export interface ResponseViewerProps {
  response: ApiResponse | null
  loading?: boolean
  error?: string | null
  className?: string
}

function getStatusColor(status: number): string {
  if (status >= 200 && status < 300) return 'text-green-600 dark:text-green-400'
  if (status >= 300 && status < 400) return 'text-blue-600 dark:text-blue-400'
  if (status >= 400 && status < 500) return 'text-orange-600 dark:text-orange-400'
  if (status >= 500) return 'text-red-600 dark:text-red-400'
  return 'text-gray-600 dark:text-gray-400'
}

export const ResponseViewer = React.forwardRef<HTMLDivElement, ResponseViewerProps>(
  ({ response, loading, error, className }, ref) => {
    if (loading) {
      return (
        <div
          ref={ref}
          className={cn(
            'flex items-center justify-center p-8 text-gray-500 dark:text-gray-400',
            className
          )}
          data-testid="response-loading"
        >
          <div className="flex items-center gap-2">
            <svg className="h-5 w-5 animate-spin" viewBox="0 0 24 24">
              <circle
                className="opacity-25"
                cx="12"
                cy="12"
                r="10"
                stroke="currentColor"
                strokeWidth="4"
                fill="none"
              />
              <path
                className="opacity-75"
                fill="currentColor"
                d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
              />
            </svg>
            <span>Sending request...</span>
          </div>
        </div>
      )
    }

    if (error) {
      return (
        <div
          ref={ref}
          className={cn(
            'p-4 rounded-md border border-red-300 bg-red-50 dark:bg-red-900/20 dark:border-red-800 text-red-700 dark:text-red-400',
            className
          )}
          data-testid="response-error"
        >
          <strong>Error:</strong> {error}
        </div>
      )
    }

    if (!response) {
      return (
        <div
          ref={ref}
          className={cn(
            'flex items-center justify-center p-8 text-gray-500 dark:text-gray-400',
            className
          )}
          data-testid="response-empty"
        >
          Send a request to see the response here
        </div>
      )
    }

    return (
      <div ref={ref} className={cn('flex flex-col gap-3', className)} data-testid="response-panel">
        <div className="flex items-center gap-4 text-sm">
          <div className="flex items-center gap-2">
            <Zap className="h-4 w-4 text-gray-500" />
            <span className="font-semibold">Status:</span>
            <span
              className={cn('font-mono font-bold', getStatusColor(response.status))}
              data-testid="response-status"
            >
              {response.status} {response.statusText}
            </span>
          </div>

          <div className="flex items-center gap-2">
            <Clock className="h-4 w-4 text-gray-500" />
            <span className="font-semibold">Time:</span>
            <span className="font-mono" data-testid="response-time">
              {formatDuration(response.timing.totalMs)}
            </span>
          </div>

          <div className="flex items-center gap-2">
            <Hash className="h-4 w-4 text-gray-500" />
            <span className="font-semibold">Size:</span>
            <span className="font-mono" data-testid="response-size">
              {formatBytes(response.size.total)}
            </span>
          </div>
        </div>

        <Tabs defaultValue="body">
          <TabsList>
            <TabsTrigger value="body">
              <FileText className="h-4 w-4 mr-1" />
              Body
            </TabsTrigger>
            <TabsTrigger value="headers">Headers</TabsTrigger>
            <TabsTrigger value="timing">
              <GanttChart className="h-4 w-4 mr-1" />
              Timing
            </TabsTrigger>
          </TabsList>

          <TabsContent value="body">
            <BodyViewer body={response.body} contentType={response.contentType} />
          </TabsContent>

          <TabsContent value="headers">
            <HeadersViewer headers={response.headers} />
          </TabsContent>

          <TabsContent value="timing">
            <TimingViewer timing={response.timing} />
          </TabsContent>
        </Tabs>
      </div>
    )
  }
)

ResponseViewer.displayName = 'ResponseViewer'
