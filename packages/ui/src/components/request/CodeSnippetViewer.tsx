import * as React from 'react'
import { Copy, Check } from 'lucide-react'
import { Button } from '../primitives/Button'
import { Select } from '../primitives/Select'
import { cn } from '../../lib/utils'
import { generateSnippet, type SnippetLanguage } from '../../lib/snippets'
import type { Request } from '../../types'

export interface CodeSnippetViewerProps {
  request: Request
  className?: string
}

const LANGUAGE_OPTIONS = [
  { value: 'curl', label: 'cURL' },
  { value: 'fetch', label: 'JavaScript (fetch)' },
  { value: 'axios', label: 'JavaScript (axios)' },
  { value: 'python', label: 'Python (requests)' },
  { value: 'go', label: 'Go (net/http)' },
  { value: 'ruby', label: 'Ruby (Net::HTTP)' },
  { value: 'powershell', label: 'PowerShell' },
  { value: 'java', label: 'Java (HttpClient)' },
]

export const CodeSnippetViewer = React.forwardRef<HTMLDivElement, CodeSnippetViewerProps>(
  ({ request, className }, ref) => {
    const [language, setLanguage] = React.useState<SnippetLanguage>('curl')
    const [copied, setCopied] = React.useState(false)

    const code = React.useMemo(() => {
      try {
        return generateSnippet(request, language)
      } catch (err) {
        return `// Error: ${err instanceof Error ? err.message : String(err)}`
      }
    }, [request, language])

    const handleCopy = async () => {
      try {
        await navigator.clipboard.writeText(code)
        setCopied(true)
        setTimeout(() => setCopied(false), 1500)
      } catch (err) {
        console.error('Failed to copy:', err)
      }
    }

    return (
      <div
        ref={ref}
        className={cn('flex flex-col gap-2 p-3', className)}
        data-testid="code-snippet-viewer"
      >
        <div className="flex items-center gap-2">
          <Select
            options={LANGUAGE_OPTIONS}
            value={language}
            onChange={(e) => setLanguage(e.target.value as SnippetLanguage)}
            className="w-56"
            aria-label="Snippet language"
          />
          <Button
            variant="outline"
            size="sm"
            onClick={handleCopy}
            className="ml-auto"
            data-testid="copy-snippet"
          >
            {copied ? (
              <>
                <Check className="h-3 w-3 mr-1" />
                Copied
              </>
            ) : (
              <>
                <Copy className="h-3 w-3 mr-1" />
                Copy
              </>
            )}
          </Button>
        </div>

        <pre
          className="p-3 rounded-md bg-gray-900 text-gray-100 overflow-auto text-xs font-mono max-h-96"
          data-testid="snippet-code"
        >
          <code>{code}</code>
        </pre>
      </div>
    )
  }
)

CodeSnippetViewer.displayName = 'CodeSnippetViewer'
