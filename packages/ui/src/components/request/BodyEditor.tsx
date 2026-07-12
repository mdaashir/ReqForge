import * as React from 'react'
import { ChevronDown, ChevronRight, AlertCircle, CheckCircle2, X } from 'lucide-react'
import { Select } from '../primitives/Select'
import { Input } from '../primitives/Input'
import { MonacoEditor, type EditorLanguage } from '../primitives/MonacoEditor'
import { cn } from '../../lib/utils'
import { useJsonSchema } from '../../hooks/useJsonSchema'
import type { BodyMode, RequestBody } from '../../types'

export interface BodyEditorProps {
  body: RequestBody
  onChange: (body: RequestBody) => void
  /** JSON schema stored alongside the request; round-tripped with the request body. */
  schema?: string
  onSchemaChange?: (schema: string) => void
  className?: string
}

const MODE_OPTIONS = [
  { value: 'none', label: 'None' },
  { value: 'json', label: 'JSON' },
  { value: 'xml', label: 'XML' },
  { value: 'text', label: 'Text' },
  { value: 'form', label: 'Form (urlencoded)' },
  { value: 'multipart', label: 'Multipart' },
  { value: 'binary', label: 'Binary' },
  { value: 'graphql', label: 'GraphQL' },
]

function defaultContentType(mode: BodyMode): string | undefined {
  switch (mode) {
    case 'json':
      return 'application/json'
    case 'xml':
      return 'application/xml'
    case 'text':
      return 'text/plain'
    case 'form':
      return 'application/x-www-form-urlencoded'
    case 'multipart':
      return 'multipart/form-data'
    case 'graphql':
      return 'application/json'
    default:
      return undefined
  }
}

const PLACEHOLDERS: Record<BodyMode, string> = {
  none: '',
  json: '{\n  "key": "value"\n}',
  xml: '<?xml version="1.0"?>\n<root>\n  <item />\n</root>',
  text: 'Raw text payload...',
  form: 'key1=value1&key2=value2',
  multipart: '(configured via form-data fields)',
  binary: '(binary file upload)',
  graphql: 'query GetUser {\n  user(id: 1) {\n    id\n    name\n  }\n}',
}

const MONACO_LANGUAGE: Record<BodyMode, EditorLanguage> = {
  none: 'plaintext',
  json: 'json',
  xml: 'xml',
  text: 'plaintext',
  form: 'plaintext',
  multipart: 'plaintext',
  binary: 'plaintext',
  graphql: 'graphql',
}

function validateJson(text: string): boolean {
  if (!text.trim()) return true
  try {
    JSON.parse(text)
    return true
  } catch {
    return false
  }
}

export const BodyEditor = React.forwardRef<HTMLDivElement, BodyEditorProps>(
  (
    { body, onChange, schema = '', onSchemaChange, className },
    ref
  ) => {
    const setMode = (mode: BodyMode) => {
      onChange({
        ...body,
        mode,
        contentType: body.contentType ?? defaultContentType(mode),
      })
    }

    const setContent = (content: string) => {
      onChange({ ...body, content })
    }

    const setContentType = (contentType: string) => {
      onChange({ ...body, contentType: contentType || undefined })
    }

    const [schemaOpen, setSchemaOpen] = React.useState(!!schema)

    const jsonValid = body.mode !== 'json' || validateJson(body.content)
    const schemaResult = useJsonSchema(
      body.mode === 'json' ? body.content : '',
      body.mode === 'json' ? schema : ''
    )

    const showSchemaControls = body.mode === 'json'

    return (
      <div
        ref={ref}
        className={cn('flex flex-col gap-3 p-3', className)}
        data-testid="body-editor"
      >
        <div className="flex items-center gap-3">
          <div className="flex-1">
            <label className="text-xs font-semibold text-gray-700 dark:text-gray-300 mb-1 block">
              Content type
            </label>
            <Select
              value={body.mode}
              onChange={(e) => setMode(e.target.value as BodyMode)}
              options={MODE_OPTIONS}
            />
          </div>
          <div className="flex-1">
            <label className="text-xs font-semibold text-gray-700 dark:text-gray-300 mb-1 block">
              Override Content-Type (optional)
            </label>
            <Input
              value={body.contentType ?? ''}
              onChange={(e) => setContentType(e.target.value)}
              placeholder={defaultContentType(body.mode) ?? 'auto'}
              className="font-mono text-sm"
            />
          </div>
        </div>

        {body.mode !== 'none' && (
          <div>
            <label className="text-xs font-semibold text-gray-700 dark:text-gray-300 mb-1 block">
              Request body
            </label>
            <MonacoEditor
              value={body.content}
              onChange={(value) => setContent(value)}
              language={MONACO_LANGUAGE[body.mode]}
              placeholder={PLACEHOLDERS[body.mode]}
              height="320px"
              className={cn(
                jsonValid && (showSchemaControls ? schemaResult.valid : true)
                  ? ''
                  : 'ring-2 ring-red-500 rounded-md'
              )}
              readOnly={body.mode === 'multipart' || body.mode === 'binary'}
            />
            {body.mode === 'json' && !jsonValid && (
              <p className="text-xs text-red-600 dark:text-red-400 mt-1 flex items-center gap-1">
                <AlertCircle className="h-3 w-3" />
                Invalid JSON
              </p>
            )}
            {showSchemaControls && schema && schemaResult.valid && body.content && (
              <p className="text-xs text-green-600 dark:text-green-400 mt-1 flex items-center gap-1">
                <CheckCircle2 className="h-3 w-3" />
                Valid against schema
              </p>
            )}
            {showSchemaControls && schema && !schemaResult.valid && schemaResult.errors.length > 0 && (
              <div className="mt-2 p-2 rounded border border-red-300 dark:border-red-800 bg-red-50 dark:bg-red-900/20">
                <p className="text-xs font-semibold text-red-700 dark:text-red-300 mb-1">
                  Schema validation failed ({schemaResult.errors.length} error{schemaResult.errors.length === 1 ? '' : 's'})
                </p>
                <ul className="text-xs text-red-600 dark:text-red-400 space-y-0.5 font-mono">
                  {schemaResult.errors.slice(0, 5).map((err, idx) => (
                    <li key={idx}>
                      <span className="font-semibold">{err.path}</span>: {err.message}
                    </li>
                  ))}
                  {schemaResult.errors.length > 5 && (
                    <li className="text-red-500">
                      …and {schemaResult.errors.length - 5} more
                    </li>
                  )}
                </ul>
              </div>
            )}
          </div>
        )}

        {showSchemaControls && (
          <div className="border border-gray-200 dark:border-gray-700 rounded-md">
            <button
              type="button"
              onClick={() => setSchemaOpen(!schemaOpen)}
              className="w-full flex items-center gap-2 px-3 py-2 text-xs font-semibold text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-800"
              data-testid="toggle-schema"
            >
              {schemaOpen ? <ChevronDown className="h-3 w-3" /> : <ChevronRight className="h-3 w-3" />}
              JSON Schema (optional validation)
              {schema && (
                <span
                  className={cn(
                    'ml-auto px-1.5 py-0.5 rounded text-[10px] font-mono',
                    schemaResult.valid
                      ? 'bg-green-100 dark:bg-green-900/40 text-green-700 dark:text-green-300'
                      : 'bg-red-100 dark:bg-red-900/40 text-red-700 dark:text-red-300'
                  )}
                >
                  {schemaResult.valid ? 'valid' : `${schemaResult.errors.length} errors`}
                </span>
              )}
            </button>
            {schemaOpen && (
              <div className="p-3 border-t border-gray-200 dark:border-gray-700">
                <MonacoEditor
                  value={schema}
                  onChange={(value) => onSchemaChange?.(value)}
                  language="json"
                  placeholder='{ "type": "object", "required": ["name"], "properties": { "name": { "type": "string" } } }'
                  height="200px"
                  className="mb-2"
                />
                {schema && (
                  <button
                    type="button"
                    onClick={() => onSchemaChange?.('')}
                    className="text-xs text-gray-500 hover:text-red-600 dark:hover:text-red-400 flex items-center gap-1"
                  >
                    <X className="h-3 w-3" />
                    Clear schema
                  </button>
                )}
              </div>
            )}
          </div>
        )}
      </div>
    )
  }
)

BodyEditor.displayName = 'BodyEditor'
