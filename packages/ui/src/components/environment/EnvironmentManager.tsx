import * as React from 'react'
import { Plus, Save, Trash2, FolderOpen } from 'lucide-react'
import { Button } from '../primitives/Button'
import { Input } from '../primitives/Input'
import { KeyValueEditor } from '../request/KeyValueEditor'
import { cn } from '../../lib/utils'
import { useEnvironmentStore } from '../../stores/environmentStore'
import { useEnvironment } from '../../hooks/useEnvironment'
import type { Environment, KeyValue } from '../../types'

export interface EnvironmentManagerProps {
  className?: string
  onClose?: () => void
}

/**
 * Environment manager UI.
 *
 * Allows users to create, edit, save, load, and delete environments.
 * Environments are persisted to disk as YAML files.
 */
export const EnvironmentManager = React.forwardRef<HTMLDivElement, EnvironmentManagerProps>(
  ({ className, onClose }, ref) => {
    const {
      environments,
      activeEnvironmentId,
      setActiveEnvironment,
      addEnvironment,
      updateEnvironment,
    } = useEnvironmentStore()
    const { save, load, list, deleteEnv } = useEnvironment()

    const current = React.useMemo(
      () => environments.find((env) => env.id === activeEnvironmentId) || null,
      [environments, activeEnvironmentId]
    )

    const [availableEnvs, setAvailableEnvs] = React.useState<string[]>([])
    const [editingName, setEditingName] = React.useState('')
    const [variables, setVariables] = React.useState<KeyValue[]>([])
    const [error, setError] = React.useState<string | null>(null)
    const [loading, setLoading] = React.useState(false)

    // Load available environments on mount
    React.useEffect(() => {
      loadAvailableEnvs()
    }, [])

    const loadAvailableEnvs = async () => {
      try {
        const envs = await list()
        setAvailableEnvs(envs)
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err))
      }
    }

    const handleNew = () => {
      setEditingName('')
      setVariables([])
      setError(null)
    }

    const handleLoad = async (name: string) => {
      setLoading(true)
      setError(null)
      try {
        const env = await load(name)
        setEditingName(env.name)
        setVariables(
          env.variables.map((v) => ({
            key: v.key,
            value: v.value,
            enabled: v.enabled,
          }))
        )
        const existing = environments.find((e) => e.name === env.name)
        if (existing) {
          setActiveEnvironment(existing.id)
          updateEnvironment(existing.id, env)
        } else {
          addEnvironment(env)
          setActiveEnvironment(env.id)
        }
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err))
      } finally {
        setLoading(false)
      }
    }

    const handleSave = async () => {
      if (!editingName.trim()) {
        setError('Environment name is required')
        return
      }

      setLoading(true)
      setError(null)
      try {
        const env: Environment = {
          id: current?.id || crypto.randomUUID(),
          name: editingName.trim(),
          variables: variables
            .filter((v) => v.key.trim())
            .map((v) => ({
              key: v.key,
              value: v.value,
              var_type: 'string' as const,
              enabled: v.enabled,
            })),
          created_at: current?.created_at || new Date().toISOString(),
          updated_at: new Date().toISOString(),
        }

        await save(env)
        if (current) {
          updateEnvironment(current.id, env)
        } else {
          addEnvironment(env)
          setActiveEnvironment(env.id)
        }
        await loadAvailableEnvs()
        setError(null)
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err))
      } finally {
        setLoading(false)
      }
    }

    const handleDelete = async (name: string) => {
      if (!confirm(`Delete environment "${name}"?`)) return

      setLoading(true)
      setError(null)
      try {
        await deleteEnv(name)
        await loadAvailableEnvs()
        if (current?.name === name) {
          setActiveEnvironment(null)
          handleNew()
        }
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err))
      } finally {
        setLoading(false)
      }
    }

    return (
      <div
        ref={ref}
        className={cn('flex flex-col gap-4 p-4 bg-white dark:bg-gray-900 rounded-lg', className)}
        data-testid="environment-manager"
      >
        <div className="flex items-center justify-between">
          <h2 className="text-lg font-semibold text-gray-900 dark:text-gray-100">
            Environment Manager
          </h2>
          {onClose && (
            <Button variant="ghost" size="sm" onClick={onClose}>
              Close
            </Button>
          )}
        </div>

        {error && (
          <div className="p-2 rounded bg-red-50 dark:bg-red-900/20 text-red-600 dark:text-red-400 text-sm">
            {error}
          </div>
        )}

        {/* Saved environments list */}
        <div className="flex flex-col gap-2">
          <div className="flex items-center justify-between">
            <label className="text-sm font-medium text-gray-700 dark:text-gray-300">
              Saved Environments
            </label>
            <Button variant="outline" size="sm" onClick={handleNew} disabled={loading}>
              <Plus className="h-3 w-3 mr-1" />
              New
            </Button>
          </div>

          <div className="flex flex-col gap-1 max-h-48 overflow-y-auto border border-gray-300 dark:border-gray-600 rounded-md p-2">
            {availableEnvs.length === 0 ? (
              <div className="text-sm text-gray-500 dark:text-gray-400 text-center py-4">
                No saved environments
              </div>
            ) : (
              availableEnvs.map((name) => (
                <div
                  key={name}
                  className="flex items-center justify-between p-2 hover:bg-gray-100 dark:hover:bg-gray-800 rounded"
                >
                  <span className="text-sm text-gray-900 dark:text-gray-100">{name}</span>
                  <div className="flex gap-1">
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => handleLoad(name)}
                      disabled={loading}
                      title="Load environment"
                    >
                      <FolderOpen className="h-3 w-3" />
                    </Button>
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => handleDelete(name)}
                      disabled={loading}
                      title="Delete environment"
                    >
                      <Trash2 className="h-3 w-3" />
                    </Button>
                  </div>
                </div>
              ))
            )}
          </div>
        </div>

        {/* Environment editor */}
        <div className="flex flex-col gap-3">
          <div className="flex items-center gap-2">
            <Input
              value={editingName}
              onChange={(e) => setEditingName(e.target.value)}
              placeholder="Environment name (e.g., production)"
              className="flex-1"
              disabled={loading}
            />
            <Button onClick={handleSave} disabled={loading || !editingName.trim()}>
              <Save className="h-3 w-3 mr-1" />
              Save
            </Button>
          </div>

          <div className="flex flex-col gap-2">
            <label className="text-sm font-medium text-gray-700 dark:text-gray-300">
              Variables
            </label>
            <KeyValueEditor
              items={variables}
              onChange={setVariables}
              keyPlaceholder="Variable name"
              valuePlaceholder="Value"
            />
          </div>
        </div>
      </div>
    )
  }
)

EnvironmentManager.displayName = 'EnvironmentManager'
