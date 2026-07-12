import * as React from 'react'
import { cn, formatDuration } from '../../lib/utils'
import type { ResponseTiming } from '../../types'

export interface TimingViewerProps {
  timing: ResponseTiming
  className?: string
}

const PHASES: { key: keyof ResponseTiming; label: string }[] = [
  { key: 'dnsMs', label: 'DNS' },
  { key: 'connectMs', label: 'Connect' },
  { key: 'tlsMs', label: 'TLS' },
  { key: 'sendMs', label: 'Send' },
  { key: 'waitMs', label: 'Wait (TTFB)' },
  { key: 'receiveMs', label: 'Receive' },
]

export const TimingViewer = React.forwardRef<HTMLDivElement, TimingViewerProps>(
  ({ timing, className }, ref) => {
    const maxVal = Math.max(
      ...PHASES.map((p) => timing[p.key]),
      1
    )

    return (
      <div
        ref={ref}
        className={cn('p-3 space-y-2', className)}
        data-testid="timing-viewer"
      >
        <div className="text-sm font-semibold mb-2">
          Total: {formatDuration(timing.totalMs)}
        </div>
        {PHASES.map(({ key, label }) => {
          const val = timing[key]
          const pct = (val / maxVal) * 100
          return (
            <div key={key} className="flex items-center gap-2 text-xs">
              <span className="w-16 text-right text-gray-500 dark:text-gray-400 font-medium">
                {label}
              </span>
              <div className="flex-1 h-4 bg-gray-100 dark:bg-gray-800 rounded-full overflow-hidden">
                <div
                  className="h-full bg-blue-500 rounded-full transition-all"
                  style={{ width: `${Math.max(pct, 2)}%` }}
                />
              </div>
              <span className="w-16 font-mono text-gray-700 dark:text-gray-300 text-right">
                {val}ms
              </span>
            </div>
          )
        })}
      </div>
    )
  }
)

TimingViewer.displayName = 'TimingViewer'
