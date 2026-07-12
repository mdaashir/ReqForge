import * as React from 'react'
import {
  Search,
  Trash2,
  RefreshCw,
  Play,
  X,
  Circle,
} from 'lucide-react'
import { Button } from '../primitives/Button'
import { Input } from '../primitives/Input'
import { cn, formatBytes, formatDuration, formatRelativeTime } from '../../lib/utils'
import { useHistoryStore, type HistoryItem } from '../../stores/historyStore'

export interface HistoryViewerProps {
  onReplay?: (entry: HistoryItem) => void
  className?: string
}

const methodColors: Record<string, string> = {
  GET: 'text-green-600 dark:text-green-400',
  POST: 'text-orange-600 dark:text-orange-400',
  PUT: 'text-blue-600 dark:text-blue-400',
  PATCH: 'text-purple-600 dark:text-purple-400',
  DELETE: 'text-red-600 dark:text-red-400',
}

type MethodFilter = 'ALL' | 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE'
type StatusFilter = 'all' | '2xx' | '3xx' | '4xx' | '5xx' | 'error'

function statusBucket(status: number): StatusFilter {
  if (status === 0) return 'error'
  if (status >= 200 && status < 300) return '2xx'
  if (status >= 300 && status < 400) return '3xx'
  if (status >= 400 && status < 500) return '4xx'
  return '5xx'
}

function statusColor(status: number): string {
  switch (statusBucket(status)) {
    case '2xx':
      return 'text-green-600 dark:text-green-400'
    case '3xx':
      return 'text-blue-600 dark:text-blue-400'
    case '4xx':
      return 'text-orange-600 dark:text-orange-400'
    case '5xx':
    case 'error':
      return 'text-red-600 dark:text-red-400'
    default:
      return 'text-gray-600 dark:text-gray-400'
  }
}

function dayBucket(ts: number): string {
  const d = new Date(ts)
  const today = new Date()
  today.setHours(0, 0, 0, 0)
  const entry = new Date(d)
  entry.setHours(0, 0, 0, 0)
  const diff = (today.getTime() - entry.getTime()) / 86_400_000
  if (diff === 0) return 'Today'
  if (diff === 1) return 'Yesterday'
  if (diff < 7) return d.toLocaleDateString(undefined, { weekday: 'long' })
  return d.toLocaleDateString()
}

export const HistoryViewer = React.forwardRef<HTMLDivElement, HistoryViewerProps>(
  ({ onReplay, className }, ref) => {
    const { items, loading, searchQuery, search, load, clear, setSearchQuery } =
      useHistoryStore()

    const [methodFilter, setMethodFilter] = React.useState<MethodFilter>('ALL')
    const [statusFilter, setStatusFilter] = React.useState<StatusFilter>('all')

    // Debounced search
    React.useEffect(() => {
      const timer = setTimeout(() => {
        if (searchQuery.trim()) {
          search(searchQuery)
        } else {
          load()
        }
      }, 200)
      return () => clearTimeout(timer)
    }, [searchQuery, search, load])

    const filtered = React.useMemo(() => {
      return items.filter((it) => {
        if (methodFilter !== 'ALL' && it.method !== methodFilter) return false
        if (statusFilter !== 'all' && statusBucket(it.status) !== statusFilter) return false
        return true
      })
    }, [items, methodFilter, statusFilter])

    const grouped = React.useMemo(() => {
      const groups: Array<[string, HistoryItem[]]> = []
      for (const item of filtered) {
        const day = dayBucket(item.timestamp)
        const tail = groups[groups.length - 1]
        if (tail && tail[0] === day) {
          tail[1].push(item)
        } else {
          groups.push([day, [item]])
        }
      }
      return groups
    }, [filtered])

    const methods: MethodFilter[] = ['ALL', 'GET', 'POST', 'PUT', 'PATCH', 'DELETE']
    const statuses: StatusFilter[] = ['all', '2xx', '3xx', '4xx', '5xx', 'error']

    const hasFilters =
      methodFilter !== 'ALL' ||
      statusFilter !== 'all' ||
      searchQuery.trim().length > 0

    return (
      <div
        ref={ref}
        className={cn('flex flex-col h-full', className)}
        data-testid="history-viewer"
      >
        <div className="flex flex-col gap-2 p-3 border-b border-gray-200 dark:border-gray-700">
          <div className="flex items-center gap-2">
            <Search className="h-4 w-4 text-gray-500" />
            <Input
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder="Search history…"
              className="flex-1"
              aria-label="Search history"
            />
            <Button variant="ghost" size="icon" onClick={() => load()} title="Refresh">
              <RefreshCw className="h-4 w-4" />
            </Button>
            <Button
              variant="ghost"
              size="icon"
              onClick={clear}
              disabled={items.length === 0}
              title="Clear history"
            >
              <Trash2 className="h-4 w-4" />
            </Button>
          </div>

          <div className="flex items-center gap-1 overflow-x-auto pb-0.5">
            {methods.map((m) => (
              <button
                key={m}
                onClick={() => setMethodFilter(m)}
                className={cn(
                  'px-2 py-0.5 rounded text-xs font-mono whitespace-nowrap transition-colors',
                  methodFilter === m
                    ? 'bg-blue-500 text-white'
                    : 'bg-gray-100 dark:bg-gray-800 text-gray-700 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-700'
                )}
                data-testid={`history-method-filter-${m}`}
              >
                {m}
              </button>
            ))}
            <span className="w-px h-4 bg-gray-300 dark:bg-gray-600 mx-1" />
            {statuses.map((s) => (
              <button
                key={s}
                onClick={() => setStatusFilter(s)}
                className={cn(
                  'px-2 py-0.5 rounded text-xs font-mono whitespace-nowrap transition-colors',
                  statusFilter === s
                    ? 'bg-blue-500 text-white'
                    : 'bg-gray-100 dark:bg-gray-800 text-gray-700 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-700'
                )}
                data-testid={`history-status-filter-${s}`}
              >
                {s}
              </button>
            ))}
            {hasFilters && (
              <button
                onClick={() => {
                  setMethodFilter('ALL')
                  setStatusFilter('all')
                  setSearchQuery('')
                }}
                className="ml-auto flex items-center gap-1 px-2 py-0.5 rounded text-xs text-gray-500 hover:text-gray-700 dark:hover:text-gray-300"
                title="Clear filters"
              >
                <X className="h-3 w-3" />
                Reset
              </button>
            )}
          </div>
        </div>

        <div className="flex-1 overflow-auto" data-testid="history-list">
          {loading && (
            <div className="p-4 text-sm text-gray-500 text-center">Loading…</div>
          )}
          {!loading && filtered.length === 0 && (
            <div className="p-8 text-sm text-gray-500 text-center">
              {items.length === 0
                ? 'No history yet. Send some requests to see them here.'
                : 'No items match the current filters.'}
            </div>
          )}
          {!loading && grouped.map(([day, dayItems]) => (
            <div key={day}>
              <div className="sticky top-0 px-3 py-1 text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider bg-white/80 dark:bg-gray-900/80 backdrop-blur z-10 border-b border-gray-100 dark:border-gray-800">
                {day} <span className="text-gray-400">({dayItems.length})</span>
              </div>
              {dayItems.map((entry) => (
                <button
                  key={entry.id}
                  onClick={() => onReplay?.(entry)}
                  className="group flex items-start gap-2 p-3 w-full text-left border-b border-gray-100 dark:border-gray-800 hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors"
                  data-testid={`history-item-${entry.id}`}
                >
                  <span
                    className={cn(
                      'font-mono font-semibold text-xs w-12 flex-shrink-0',
                      methodColors[entry.method] || 'text-gray-600 dark:text-gray-400'
                    )}
                  >
                    {entry.method}
                  </span>
                  <div className="flex-1 min-w-0">
                    <div className="text-sm truncate text-gray-900 dark:text-gray-100">
                      {entry.url}
                    </div>
                    <div className="flex items-center gap-3 text-xs text-gray-500 mt-0.5">
                      <span className="flex items-center gap-1">
                        <Circle
                          className={cn(
                            'h-2 w-2 fill-current',
                            statusColor(entry.status)
                          )}
                        />
                        <span
                          className={cn(
                            'font-mono font-semibold',
                            statusColor(entry.status)
                          )}
                        >
                          {entry.status || 'ERR'}
                        </span>
                      </span>
                      <span>{formatDuration(entry.durationMs)}</span>
                      <span>{formatBytes(entry.sizeBytes)}</span>
                      <span className="ml-auto">{formatRelativeTime(entry.timestamp)}</span>
                    </div>
                  </div>
                  <button
                    onClick={(e) => {
                      e.stopPropagation()
                      onReplay?.(entry)
                    }}
                    className="opacity-0 group-hover:opacity-100 p-1 rounded hover:bg-blue-100 dark:hover:bg-blue-900/40 text-blue-600 dark:text-blue-400 transition-opacity"
                    title="Replay request"
                    data-testid={`history-replay-${entry.id}`}
                  >
                    <Play className="h-3 w-3" />
                  </button>
                </button>
              ))}
            </div>
          ))}
        </div>
      </div>
    )
  }
)

HistoryViewer.displayName = 'HistoryViewer'
