import * as React from 'react'
import { Plus, Settings2 } from 'lucide-react'
import { Button } from '../primitives/Button'
import { Input } from '../primitives/Input'
import { cn } from '../../lib/utils'
import type { Environment } from '../../types'

export interface EnvironmentSelectorProps {
  environments: Environment[]
  activeEnvironmentId: string | null
  onSelectEnvironment: (id: string | null) => void
  onCreateEnvironment: (name: string) => void
  onEditEnvironment?: (id: string) => void
  className?: string
}

export const EnvironmentSelector = React.forwardRef<HTMLDivElement, EnvironmentSelectorProps>(
  (
    {
      environments,
      activeEnvironmentId,
      onSelectEnvironment,
      onCreateEnvironment,
      onEditEnvironment,
      className,
    },
    ref
  ) => {
    const [isCreating, setIsCreating] = React.useState(false)
    const [newName, setNewName] = React.useState('')
    const inputRef = React.useRef<HTMLInputElement>(null)

    React.useEffect(() => {
      if (isCreating) {
        inputRef.current?.focus()
      }
    }, [isCreating])

    const handleCreate = () => {
      if (newName.trim()) {
        onCreateEnvironment(newName.trim())
        setNewName('')
        setIsCreating(false)
      }
    }

    return (
      <div
        ref={ref}
        className={cn('flex items-center gap-1', className)}
        data-testid="environment-selector"
      >
        <select
          value={activeEnvironmentId || ''}
          onChange={(e) => onSelectEnvironment(e.target.value || null)}
          className="h-8 px-2 rounded-md border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 text-sm text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
          aria-label="Active environment"
        >
          <option value="">No environment</option>
          {environments.map((env) => (
            <option key={env.id} value={env.id}>
              {env.name}
            </option>
          ))}
        </select>

        {onEditEnvironment && activeEnvironmentId && (
          <Button
            variant="ghost"
            size="icon"
            className="h-8 w-8"
            onClick={() => onEditEnvironment(activeEnvironmentId)}
            title="Edit environment"
          >
            <Settings2 className="h-4 w-4" />
          </Button>
        )}

        {isCreating ? (
          <div className="flex items-center gap-1">
            <Input
              ref={inputRef}
              value={newName}
              onChange={(e) => setNewName(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter') handleCreate()
                if (e.key === 'Escape') {
                  setIsCreating(false)
                  setNewName('')
                }
              }}
              placeholder="Environment name"
              className="h-8 w-40"
            />
            <Button size="sm" onClick={handleCreate}>
              Add
            </Button>
          </div>
        ) : (
          <Button
            variant="ghost"
            size="icon"
            className="h-8 w-8"
            onClick={() => setIsCreating(true)}
            title="New environment"
          >
            <Plus className="h-4 w-4" />
          </Button>
        )}
      </div>
    )
  }
)

EnvironmentSelector.displayName = 'EnvironmentSelector'
