import * as React from 'react'
import { Plus, Trash2, Play } from 'lucide-react'
import { Button } from '../primitives/Button'
import { Input } from '../primitives/Input'
import { Select } from '../primitives/Select'
import { TestResults } from './TestResults'
import { cn } from '../../lib/utils'
import { useTestRunner } from '../../hooks/useTestRunner'
import type { ApiResponse, Assertion } from '../../types'

export interface TestEditorProps {
  response: ApiResponse | null
  onRunTests?: () => void
  className?: string
}

const ASSERTION_TYPES = [
  { value: 'status_code', label: 'Status code' },
  { value: 'response_time', label: 'Response time' },
  { value: 'body_contains', label: 'Body contains' },
  { value: 'body_matches', label: 'Body matches regex' },
  { value: 'header_equals', label: 'Header equals' },
  { value: 'header_contains', label: 'Header contains' },
  { value: 'json_path', label: 'JSON path' },
  { value: 'content_type', label: 'Content-Type' },
]

export const TestEditor = React.forwardRef<HTMLDivElement, TestEditorProps>(
  ({ response, onRunTests, className }, ref) => {
    const {
      assertions,
      results,
      loading,
      addQuickAssertion,
      removeAssertion,
      updateAssertion,
      runTests,
    } = useTestRunner()

    const handleRun = async () => {
      if (response) {
        await runTests(response)
        onRunTests?.()
      }
    }

    return (
      <div ref={ref} className={cn('flex flex-col gap-3 p-3', className)} data-testid="test-editor">
        <div className="flex items-center justify-between">
          <h3 className="text-sm font-semibold text-gray-900 dark:text-gray-100">
            Assertions
          </h3>
          <Button
            onClick={handleRun}
            size="sm"
            disabled={!response || assertions.length === 0}
            data-testid="run-tests-button"
          >
            <Play className="h-3 w-3 mr-1" />
            Run tests
          </Button>
        </div>

        {assertions.length === 0 ? (
          <div className="text-sm text-gray-500 dark:text-gray-400 text-center py-4 border border-dashed border-gray-300 dark:border-gray-700 rounded">
            No assertions yet. Add one below.
          </div>
        ) : (
          <div className="flex flex-col gap-2">
            {assertions.map((a) => (
              <AssertionRow
                key={a.id}
                assertion={a}
                onUpdate={(patch) => updateAssertion(a.id, patch)}
                onRemove={() => removeAssertion(a.id)}
              />
            ))}
          </div>
        )}

        <div className="flex items-center gap-2">
          <Select
            options={ASSERTION_TYPES}
            onChange={(e) => addQuickAssertion(e.target.value as Assertion['config']['type'])}
            className="w-48"
            defaultValue=""
            aria-label="Assertion type"
          />
          <Button
            variant="outline"
            size="sm"
            onClick={() => {
              const select = (document.querySelector('[aria-label="Assertion type"]') as HTMLSelectElement)
              if (select?.value) {
                addQuickAssertion(select.value as Assertion['config']['type'])
                select.value = ''
              }
            }}
          >
            <Plus className="h-3 w-3 mr-1" />
            Add assertion
          </Button>
        </div>

        {results && (
          <div className="mt-4">
            <TestResults results={results} loading={loading} />
          </div>
        )}
      </div>
    )
  }
)

TestEditor.displayName = 'TestEditor'

interface AssertionRowProps {
  assertion: Assertion
  onUpdate: (patch: Partial<Assertion>) => void
  onRemove: () => void
}

function AssertionRow({ assertion, onUpdate, onRemove }: AssertionRowProps) {
  const updateConfig = (patch: Partial<Assertion['config']>) => {
    onUpdate({ config: { ...assertion.config, ...patch } as Assertion['config'] })
  }

  return (
    <div className="flex items-start gap-2 p-2 rounded border border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-900">
      <div className="flex-1 flex flex-col gap-2">
        <Input
          value={assertion.name}
          onChange={(e) => onUpdate({ name: e.target.value })}
          placeholder="Assertion name"
          className="text-sm"
        />

        <AssertionFields
          config={assertion.config}
          onChange={(patch) => updateConfig(patch)}
        />
      </div>

      <Button
        variant="ghost"
        size="icon"
        className="h-8 w-8 mt-1"
        onClick={onRemove}
        title="Remove assertion"
      >
        <Trash2 className="h-4 w-4" />
      </Button>
    </div>
  )
}

function AssertionFields({
  config,
  onChange,
}: {
  config: Assertion['config']
  onChange: (patch: Partial<Assertion['config']>) => void
}) {
  switch (config.type) {
    case 'status_code':
      return (
        <div className="flex items-center gap-2 text-sm">
          <span className="text-gray-600 dark:text-gray-400">Expected status</span>
          <Input
            type="number"
            value={config.expected}
            onChange={(e) => onChange({ expected: parseInt(e.target.value, 10) || 0 })}
            className="w-24 font-mono"
          />
        </div>
      )
    case 'response_time':
      return (
        <div className="flex items-center gap-2 text-sm">
          <span className="text-gray-600 dark:text-gray-400">Max (ms)</span>
          <Input
            type="number"
            value={config.maxMs}
            onChange={(e) => onChange({ maxMs: parseInt(e.target.value, 10) || 0 })}
            className="w-24 font-mono"
          />
        </div>
      )
    case 'body_contains':
      return (
        <Input
          value={config.substring}
          onChange={(e) => onChange({ substring: e.target.value })}
          placeholder="substring"
          className="text-sm font-mono"
        />
      )
    case 'body_matches':
      return (
        <Input
          value={config.pattern}
          onChange={(e) => onChange({ pattern: e.target.value })}
          placeholder="regex pattern"
          className="text-sm font-mono"
        />
      )
    case 'header_equals':
      return (
        <div className="flex items-center gap-2">
          <Input
            value={config.header}
            onChange={(e) => onChange({ header: e.target.value })}
            placeholder="Header name"
            className="text-sm font-mono flex-1"
          />
          <Input
            value={config.expected}
            onChange={(e) => onChange({ expected: e.target.value })}
            placeholder="Expected value"
            className="text-sm font-mono flex-1"
          />
        </div>
      )
    case 'header_contains':
      return (
        <div className="flex items-center gap-2">
          <Input
            value={config.header}
            onChange={(e) => onChange({ header: e.target.value })}
            placeholder="Header name"
            className="text-sm font-mono flex-1"
          />
          <Input
            value={config.substring}
            onChange={(e) => onChange({ substring: e.target.value })}
            placeholder="substring"
            className="text-sm font-mono flex-1"
          />
        </div>
      )
    case 'json_path':
      return (
        <div className="flex items-center gap-2">
          <Input
            value={config.path}
            onChange={(e) => onChange({ path: e.target.value })}
            placeholder="$.data.id"
            className="text-sm font-mono flex-1"
          />
          <Input
            value={config.expected}
            onChange={(e) => onChange({ expected: e.target.value })}
            placeholder="Expected value"
            className="text-sm font-mono flex-1"
          />
        </div>
      )
    case 'content_type':
      return (
        <Input
          value={config.expected}
          onChange={(e) => onChange({ expected: e.target.value })}
          placeholder="application/json"
          className="text-sm font-mono"
        />
      )
  }
}
