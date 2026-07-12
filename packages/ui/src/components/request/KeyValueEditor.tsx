import * as React from 'react'
import { Plus, Trash2 } from 'lucide-react'
import { Button } from '../primitives/Button'
import { Input } from '../primitives/Input'
import { cn } from '../../lib/utils'
import type { KeyValue } from '../../types'

export interface KeyValueEditorProps {
  items: KeyValue[]
  onChange: (items: KeyValue[]) => void
  keyPlaceholder?: string
  valuePlaceholder?: string
  className?: string
}

export const KeyValueEditor = React.forwardRef<HTMLDivElement, KeyValueEditorProps>(
  (
    {
      items,
      onChange,
      keyPlaceholder = 'Key',
      valuePlaceholder = 'Value',
      className,
    },
    ref
  ) => {
    const updateAt = (idx: number, patch: Partial<KeyValue>) => {
      const next = items.map((item, i) => (i === idx ? { ...item, ...patch } : item))
      onChange(next)
    }

    const removeAt = (idx: number) => {
      onChange(items.filter((_, i) => i !== idx))
    }

    const addRow = () => {
      onChange([
        ...items,
        { key: '', value: '', enabled: true, description: undefined },
      ])
    }

    return (
      <div ref={ref} className={cn('flex flex-col gap-2 p-3', className)} data-testid="kv-editor">
        <div className="grid grid-cols-[auto_1fr_1fr_auto] gap-2 items-center text-xs font-semibold text-gray-700 dark:text-gray-300 px-1">
          <div className="w-8"></div>
          <div>{keyPlaceholder}</div>
          <div>{valuePlaceholder}</div>
          <div className="w-8"></div>
        </div>

        {items.length === 0 && (
          <div className="text-sm text-gray-500 dark:text-gray-400 text-center py-4">
            No entries. Click "Add" to create one.
          </div>
        )}

        {items.map((item, idx) => (
          <div
            key={idx}
            className="grid grid-cols-[auto_1fr_1fr_auto] gap-2 items-center"
          >
            <input
              type="checkbox"
              checked={item.enabled}
              onChange={(e) => updateAt(idx, { enabled: e.target.checked })}
              className="h-4 w-4 justify-self-center"
              aria-label={`Enable row ${idx + 1}`}
            />
            <Input
              value={item.key}
              onChange={(e) => updateAt(idx, { key: e.target.value })}
              placeholder={keyPlaceholder}
              className="font-mono text-sm"
            />
            <Input
              value={item.value}
              onChange={(e) => updateAt(idx, { value: e.target.value })}
              placeholder={valuePlaceholder}
              className="font-mono text-sm"
            />
            <Button
              variant="ghost"
              size="icon"
              onClick={() => removeAt(idx)}
              className="h-8 w-8"
              title="Remove row"
            >
              <Trash2 className="h-4 w-4" />
            </Button>
          </div>
        ))}

        <div>
          <Button variant="outline" size="sm" onClick={addRow}>
            <Plus className="h-4 w-4 mr-1" />
            Add
          </Button>
        </div>
      </div>
    )
  }
)

KeyValueEditor.displayName = 'KeyValueEditor'
