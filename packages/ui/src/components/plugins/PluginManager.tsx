import * as React from 'react'
import { Power, PowerOff, RefreshCw, Puzzle, AlertTriangle } from 'lucide-react'
import { Button } from '../primitives/Button'
import { cn } from '../../lib/utils'
import { usePlugins, type PluginInfo } from '../../hooks/usePlugins'

export interface PluginManagerProps {
  className?: string
  onClose?: () => void
}

/**
 * UI for browsing, enabling, and disabling ReqForge plugins.
 *
 * Plugins live in `<workspace>/plugins/<id>/plugin.toml` and are loaded
 * into the wasmtime sandbox by the Rust side.
 */
export const PluginManager = React.forwardRef<HTMLDivElement, PluginManagerProps>(
  ({ className, onClose }, ref) => {
    const { plugins, loading, error, reload, enable, disable } = usePlugins()

    return (
      <div
        ref={ref}
        className={cn(
          'flex flex-col gap-3 p-4 bg-white dark:bg-gray-900 rounded-lg',
          className
        )}
        data-testid="plugin-manager"
      >
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Puzzle className="h-4 w-4" />
            <h2 className="text-lg font-semibold text-gray-900 dark:text-gray-100">
              Plugins
            </h2>
          </div>
          <div className="flex gap-1">
            <Button variant="ghost" size="icon" onClick={reload} disabled={loading} title="Reload">
              <RefreshCw className={cn('h-4 w-4', loading && 'animate-spin')} />
            </Button>
            {onClose && (
              <Button variant="ghost" size="sm" onClick={onClose}>
                Close
              </Button>
            )}
          </div>
        </div>

        {error && (
          <div className="flex items-center gap-2 p-2 rounded bg-red-50 dark:bg-red-900/20 text-red-600 dark:text-red-400 text-sm">
            <AlertTriangle className="h-4 w-4 flex-shrink-0" />
            <span className="truncate">{error}</span>
          </div>
        )}

        <p className="text-xs text-gray-500 dark:text-gray-400">
          Drop a <code>plugin.toml</code> + <code>plugin.wasm</code> into your
          workspace's <code>plugins/</code> directory, then reload.
        </p>

        <div className="flex flex-col gap-2 max-h-96 overflow-y-auto">
          {plugins.length === 0 && !loading && (
            <div className="p-6 text-sm text-gray-500 dark:text-gray-400 text-center border border-dashed border-gray-300 dark:border-gray-700 rounded">
              No plugins installed.
            </div>
          )}
          {plugins.map((plugin) => (
            <PluginRow
              key={plugin.id}
              plugin={plugin}
              onEnable={() => enable(plugin.id)}
              onDisable={() => disable(plugin.id)}
            />
          ))}
        </div>
      </div>
    )
  }
)

PluginManager.displayName = 'PluginManager'

interface PluginRowProps {
  plugin: PluginInfo
  onEnable: () => void
  onDisable: () => void
}

const PluginRow: React.FC<PluginRowProps> = ({ plugin, onEnable, onDisable }) => {
  const enabled = plugin.permissions.length > 0 // naive heuristic; real impl tracks separately
  return (
    <div
      className="flex items-start gap-3 p-3 border border-gray-200 dark:border-gray-700 rounded-md hover:bg-gray-50 dark:hover:bg-gray-800/50"
      data-testid={`plugin-${plugin.id}`}
    >
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <span className="font-semibold text-sm text-gray-900 dark:text-gray-100">
            {plugin.name}
          </span>
          <span className="text-xs font-mono text-gray-500">{plugin.version}</span>
        </div>
        {plugin.description && (
          <p className="text-xs text-gray-600 dark:text-gray-400 mt-0.5">
            {plugin.description}
          </p>
        )}
        <div className="text-[10px] text-gray-500 font-mono mt-1">{plugin.id}</div>
        {plugin.permissions.length > 0 && (
          <div className="flex gap-1 mt-2 flex-wrap">
            {plugin.permissions.map((p) => (
              <span
                key={p}
                className="px-1.5 py-0.5 rounded text-[10px] font-mono bg-gray-100 dark:bg-gray-800 text-gray-700 dark:text-gray-300"
              >
                {p}
              </span>
            ))}
          </div>
        )}
      </div>
      <Button
        variant={enabled ? 'ghost' : 'default'}
        size="sm"
        onClick={enabled ? onDisable : onEnable}
        data-testid={`plugin-toggle-${plugin.id}`}
      >
        {enabled ? (
          <>
            <PowerOff className="h-3 w-3 mr-1" />
            Disable
          </>
        ) : (
          <>
            <Power className="h-3 w-3 mr-1" />
            Enable
          </>
        )}
      </Button>
    </div>
  )
}
