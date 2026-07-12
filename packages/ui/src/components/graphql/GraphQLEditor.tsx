import * as React from 'react'
import { Send, Code2, FileJson } from 'lucide-react'
import { Button } from '../primitives/Button'
import { Input } from '../primitives/Input'
import { Tabs, TabsList, TabsTrigger, TabsContent } from '../primitives/Tabs'
import { cn } from '../../lib/utils'

export interface GraphQLEditorProps {
  endpoint: string
  query: string
  variables: string
  headers: Array<{ key: string; value: string; enabled: boolean }>
  loading?: boolean
  onEndpointChange: (endpoint: string) => void
  onQueryChange: (query: string) => void
  onVariablesChange: (variables: string) => void
  onHeadersChange: (headers: Array<{ key: string; value: string; enabled: boolean }>) => void
  onSend: () => void
  className?: string
}

const DEFAULT_QUERY = `# Welcome to the GraphQL editor
# Example query:
query GetUsers {
  users {
    id
    name
    email
  }
}`

export const GraphQLEditor = React.forwardRef<HTMLDivElement, GraphQLEditorProps>(
  (
    {
      endpoint,
      query,
      variables,
      headers,
      loading,
      onEndpointChange,
      onQueryChange,
      onVariablesChange,
      onHeadersChange,
      onSend,
      className,
    },
    ref
  ) => {
    const variablesValid = React.useMemo(() => {
      if (!variables.trim()) return true
      try {
        JSON.parse(variables)
        return true
      } catch {
        return false
      }
    }, [variables])

    const detectOperationType = (q: string): string => {
      const trimmed = q.trimStart()
      if (trimmed.startsWith('mutation')) return 'mutation'
      if (trimmed.startsWith('subscription')) return 'subscription'
      return 'query'
    }

    const opType = detectOperationType(query)

    return (
      <div ref={ref} className={cn('flex flex-col gap-3 p-3', className)} data-testid="graphql-editor">
        <div className="flex items-center gap-2">
          <Input
            type="url"
            value={endpoint}
            onChange={(e) => onEndpointChange(e.target.value)}
            placeholder="https://api.example.com/graphql"
            className="flex-1 font-mono"
            aria-label="GraphQL endpoint"
          />
          <Button onClick={onSend} loading={loading} disabled={!endpoint || !variablesValid}>
            <Send className="h-4 w-4" />
            Send
          </Button>
        </div>

        <div className="flex items-center gap-2 text-xs">
          <span
            className={cn(
              'px-2 py-0.5 rounded font-semibold uppercase',
              opType === 'query' && 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400',
              opType === 'mutation' && 'bg-orange-100 text-orange-700 dark:bg-orange-900/30 dark:text-orange-400',
              opType === 'subscription' && 'bg-purple-100 text-purple-700 dark:bg-purple-900/30 dark:text-purple-400'
            )}
          >
            {opType}
          </span>
          <span className="text-gray-500 dark:text-gray-400">
            Detected operation type
          </span>
        </div>

        <Tabs defaultValue="query">
          <TabsList>
            <TabsTrigger value="query">
              <Code2 className="h-4 w-4 mr-1" />
              Query
            </TabsTrigger>
            <TabsTrigger value="variables">
              <FileJson className="h-4 w-4 mr-1" />
              Variables
            </TabsTrigger>
            <TabsTrigger value="headers">Headers</TabsTrigger>
          </TabsList>

          <TabsContent value="query">
            <textarea
              value={query}
              onChange={(e) => onQueryChange(e.target.value)}
              placeholder={DEFAULT_QUERY}
              className={cn(
                'w-full h-64 p-3 rounded-md border border-gray-300 dark:border-gray-600',
                'bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100',
                'font-mono text-sm resize-y',
                'focus:outline-none focus:ring-2 focus:ring-blue-500'
              )}
              spellCheck={false}
              data-testid="graphql-query"
            />
          </TabsContent>

          <TabsContent value="variables">
            <textarea
              value={variables}
              onChange={(e) => onVariablesChange(e.target.value)}
              placeholder='{ "id": 1 }'
              className={cn(
                'w-full h-64 p-3 rounded-md border font-mono text-sm resize-y',
                'bg-white dark:bg-gray-800',
                'focus:outline-none focus:ring-2',
                variablesValid
                  ? 'border-gray-300 dark:border-gray-600 text-gray-900 dark:text-gray-100 focus:ring-blue-500'
                  : 'border-red-500 text-red-600 focus:ring-red-500'
              )}
              spellCheck={false}
              data-testid="graphql-variables"
            />
            {!variablesValid && (
              <p className="text-xs text-red-600 dark:text-red-400 mt-1">
                Invalid JSON
              </p>
            )}
          </TabsContent>

          <TabsContent value="headers">
            <div className="flex flex-col gap-2">
              {headers.map((h, idx) => (
                <div key={idx} className="flex items-center gap-2">
                  <input
                    type="checkbox"
                    checked={h.enabled}
                    onChange={(e) => {
                      const next = [...headers]
                      next[idx] = { ...h, enabled: e.target.checked }
                      onHeadersChange(next)
                    }}
                    className="h-4 w-4"
                  />
                  <Input
                    placeholder="Header name"
                    value={h.key}
                    onChange={(e) => {
                      const next = [...headers]
                      next[idx] = { ...h, key: e.target.value }
                      onHeadersChange(next)
                    }}
                    className="flex-1"
                  />
                  <Input
                    placeholder="Value"
                    value={h.value}
                    onChange={(e) => {
                      const next = [...headers]
                      next[idx] = { ...h, value: e.target.value }
                      onHeadersChange(next)
                    }}
                    className="flex-1"
                  />
                  <Button
                    variant="ghost"
                    size="icon"
                    onClick={() => {
                      const next = headers.filter((_, i) => i !== idx)
                      onHeadersChange(next)
                    }}
                  >
                    ×
                  </Button>
                </div>
              ))}
              <Button
                variant="outline"
                size="sm"
                onClick={() =>
                  onHeadersChange([
                    ...headers,
                    { key: '', value: '', enabled: true },
                  ])
                }
              >
                + Add header
              </Button>
            </div>
          </TabsContent>
        </Tabs>
      </div>
    )
  }
)

GraphQLEditor.displayName = 'GraphQLEditor'
