import * as React from 'react'
import { Search } from 'lucide-react'
import { cn } from '../../lib/utils'
import { fuzzyRank } from '../../lib/fuzzy'

export interface Command {
  id: string
  label: string
  description?: string
  shortcut?: string
  group?: string
  icon?: React.ReactNode
  keywords?: string[]
  perform: () => void | Promise<void>
}

export interface CommandPaletteProps {
  open: boolean
  commands: Command[]
  onClose: () => void
  className?: string
}

const haystack = (cmd: Command) =>
  `${cmd.label} ${cmd.description ?? ''} ${cmd.group ?? ''} ${(cmd.keywords ?? []).join(' ')}`.trim()

export const CommandPalette = React.forwardRef<HTMLDivElement, CommandPaletteProps>(
  ({ open, commands, onClose, className }, ref) => {
    const [query, setQuery] = React.useState('')
    const [activeIdx, setActiveIdx] = React.useState(0)
    const inputRef = React.useRef<HTMLInputElement>(null)

    React.useEffect(() => {
      if (open) {
        setQuery('')
        setActiveIdx(0)
        // Focus on next tick so the input is mounted
        setTimeout(() => inputRef.current?.focus(), 0)
      }
    }, [open])

    const filtered = React.useMemo(() => {
      return fuzzyRank(commands, query, haystack)
    }, [commands, query])

    // Group by group label, preserving order
    const grouped = React.useMemo(() => {
      const map = new Map<string, Command[]>()
      for (const cmd of filtered) {
        const group = cmd.group ?? 'General'
        if (!map.has(group)) map.set(group, [])
        map.get(group)!.push(cmd)
      }
      return Array.from(map.entries())
    }, [filtered])

    // Keep active index in range
    React.useEffect(() => {
      if (activeIdx >= filtered.length) {
        setActiveIdx(Math.max(0, filtered.length - 1))
      }
    }, [filtered.length, activeIdx])

    if (!open) return null

    const runCommand = async (cmd: Command) => {
      try {
        await cmd.perform()
      } finally {
        onClose()
      }
    }

    const handleKeyDown = (e: React.KeyboardEvent) => {
      if (e.key === 'ArrowDown') {
        e.preventDefault()
        setActiveIdx((i) => Math.min(filtered.length - 1, i + 1))
      } else if (e.key === 'ArrowUp') {
        e.preventDefault()
        setActiveIdx((i) => Math.max(0, i - 1))
      } else if (e.key === 'Enter') {
        e.preventDefault()
        const cmd = filtered[activeIdx]
        if (cmd) runCommand(cmd)
      } else if (e.key === 'Escape') {
        e.preventDefault()
        onClose()
      }
    }

    // Build a flat index across groups so activeIdx maps correctly
    const flatIndex = new Map<string, number>()
    let counter = 0
    for (const [, cmds] of grouped) {
      for (const cmd of cmds) {
        flatIndex.set(cmd.id, counter++)
      }
    }

    return (
      <div
        ref={ref}
        className="fixed inset-0 z-50 flex items-start justify-center pt-[15vh] bg-black/50"
        onClick={onClose}
        data-testid="command-palette"
      >
        <div
          className={cn(
            'w-full max-w-xl mx-4 rounded-lg shadow-2xl overflow-hidden',
            'bg-white dark:bg-gray-900 border border-gray-200 dark:border-gray-700',
            className
          )}
          onClick={(e) => e.stopPropagation()}
        >
          <div className="flex items-center gap-2 px-3 border-b border-gray-200 dark:border-gray-700">
            <Search className="h-4 w-4 text-gray-500" />
            <input
              ref={inputRef}
              type="text"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder="Type a command…"
              className="flex-1 py-3 bg-transparent outline-none text-sm text-gray-900 dark:text-gray-100 placeholder:text-gray-500"
              aria-label="Command palette input"
              data-testid="command-input"
            />
          </div>

          <div className="max-h-80 overflow-auto py-1">
            {filtered.length === 0 && (
              <div className="px-4 py-8 text-sm text-gray-500 dark:text-gray-400 text-center">
                No matching commands
              </div>
            )}

            {grouped.map(([group, cmds]) => (
              <div key={group} className="mb-1">
                <div className="px-3 py-1 text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                  {group}
                </div>
                {cmds.map((cmd) => {
                  const flatIdx = flatIndex.get(cmd.id) ?? 0
                  const isActive = flatIdx === activeIdx
                  return (
                    <button
                      key={cmd.id}
                      onMouseEnter={() => setActiveIdx(flatIdx)}
                      onClick={() => runCommand(cmd)}
                      className={cn(
                        'w-full flex items-center gap-2 px-3 py-2 text-left text-sm',
                        isActive
                          ? 'bg-blue-100 dark:bg-blue-900/40 text-blue-900 dark:text-blue-100'
                          : 'hover:bg-gray-100 dark:hover:bg-gray-800 text-gray-900 dark:text-gray-100'
                      )}
                      data-testid={`command-${cmd.id}`}
                    >
                      {cmd.icon && <span className="flex-shrink-0">{cmd.icon}</span>}
                      <span className="flex-1 truncate">
                        {cmd.label}
                        {cmd.description && (
                          <span className="text-xs text-gray-500 dark:text-gray-400 ml-2">
                            {cmd.description}
                          </span>
                        )}
                      </span>
                      {cmd.shortcut && (
                        <kbd className="flex-shrink-0 px-1.5 py-0.5 rounded text-xs font-mono bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300">
                          {cmd.shortcut}
                        </kbd>
                      )}
                    </button>
                  )
                })}
              </div>
            ))}
          </div>
        </div>
      </div>
    )
  }
)

CommandPalette.displayName = 'CommandPalette'
