import * as React from 'react'
import { Upload, X } from 'lucide-react'
import { Button } from '../primitives/Button'
import { Select } from '../primitives/Select'
import { cn } from '../../lib/utils'

export interface ImportDialogProps {
  open: boolean
  onClose: () => void
  onImport: (content: string, format: 'auto' | 'postman' | 'curl') => void | Promise<void>
  className?: string
}

const FORMAT_OPTIONS = [
  { value: 'auto', label: 'Auto-detect' },
  { value: 'postman', label: 'Postman v2.1 (JSON)' },
  { value: 'curl', label: 'cURL command' },
]

const SAMPLE_CURL = `curl -X POST https://api.example.com/users \\
  -H "Content-Type: application/json" \\
  -H "Authorization: Bearer my-token" \\
  -d '{"name": "Alice", "email": "alice@example.com"}'`

const SAMPLE_POSTMAN = `{
  "info": {
    "name": "Sample API",
    "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
  },
  "item": [
    {
      "name": "Get Users",
      "request": {
        "method": "GET",
        "url": "https://api.example.com/users"
      }
    }
  ]
}`

export const ImportDialog = React.forwardRef<HTMLDivElement, ImportDialogProps>(
  ({ open, onClose, onImport, className }, ref) => {
    const [content, setContent] = React.useState('')
    const [format, setFormat] = React.useState<'auto' | 'postman' | 'curl'>('auto')
    const [busy, setBusy] = React.useState(false)
    const [error, setError] = React.useState<string | null>(null)
    const inputRef = React.useRef<HTMLInputElement>(null)

    React.useEffect(() => {
      if (!open) {
        setContent('')
        setFormat('auto')
        setError(null)
      }
    }, [open])

    if (!open) return null

    const handleFile = (file: File) => {
      const reader = new FileReader()
      reader.onload = () => {
        setContent(String(reader.result ?? ''))
      }
      reader.readAsText(file)
    }

    const handleSubmit = async () => {
      if (!content.trim()) return
      setBusy(true)
      setError(null)
      try {
        await onImport(content, format)
        onClose()
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err))
      } finally {
        setBusy(false)
      }
    }

    return (
      <div
        ref={ref}
        className="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
        data-testid="import-dialog"
      >
        <div
          className={cn(
            'w-full max-w-2xl max-h-[80vh] flex flex-col rounded-lg shadow-xl',
            'bg-white dark:bg-gray-900 text-gray-900 dark:text-gray-100',
            className
          )}
        >
          <header className="flex items-center justify-between p-4 border-b border-gray-200 dark:border-gray-700">
            <div className="flex items-center gap-2">
              <Upload className="h-5 w-5" />
              <h2 className="font-semibold text-lg">Import collection</h2>
            </div>
            <Button variant="ghost" size="icon" onClick={onClose}>
              <X className="h-4 w-4" />
            </Button>
          </header>

          <div className="flex-1 overflow-auto p-4 flex flex-col gap-4">
            <div>
              <label className="text-xs font-semibold mb-1 block">Format</label>
              <Select
                options={FORMAT_OPTIONS}
                value={format}
                onChange={(e) => setFormat(e.target.value as typeof format)}
              />
            </div>

            <div>
              <label className="text-xs font-semibold mb-1 block">
                Paste content or pick a file
              </label>
              <div className="flex items-center gap-2 mb-2">
                <input
                  ref={inputRef}
                  type="file"
                  accept=".json,.txt,.http,application/json,text/plain"
                  onChange={(e) => {
                    const file = e.target.files?.[0]
                    if (file) handleFile(file)
                  }}
                  className="text-sm"
                />
              </div>
              <textarea
                value={content}
                onChange={(e) => setContent(e.target.value)}
                placeholder="Paste a Postman v2.1 JSON collection or a cURL command here…"
                className={cn(
                  'w-full h-64 p-3 rounded-md border font-mono text-xs resize-y',
                  'bg-white dark:bg-gray-800 border-gray-300 dark:border-gray-600',
                  'focus:outline-none focus:ring-2 focus:ring-blue-500'
                )}
                spellCheck={false}
                data-testid="import-content"
              />
              <div className="flex items-center gap-2 mt-2 text-xs">
                <button
                  type="button"
                  className="text-blue-600 hover:underline"
                  onClick={() => setContent(SAMPLE_CURL)}
                >
                  Insert sample cURL
                </button>
                <span className="text-gray-400">·</span>
                <button
                  type="button"
                  className="text-blue-600 hover:underline"
                  onClick={() => setContent(SAMPLE_POSTMAN)}
                >
                  Insert sample Postman
                </button>
              </div>
            </div>

            {error && (
              <div
                className="p-3 rounded border border-red-300 bg-red-50 dark:bg-red-900/20 dark:border-red-800 text-red-700 dark:text-red-400 text-sm"
                data-testid="import-error"
              >
                {error}
              </div>
            )}
          </div>

          <footer className="flex items-center justify-end gap-2 p-4 border-t border-gray-200 dark:border-gray-700">
            <Button variant="ghost" onClick={onClose} disabled={busy}>
              Cancel
            </Button>
            <Button onClick={handleSubmit} loading={busy} disabled={!content.trim()}>
              Import
            </Button>
          </footer>
        </div>
      </div>
    )
  }
)

ImportDialog.displayName = 'ImportDialog'
