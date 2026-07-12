import * as React from 'react'
import { cn } from '../../lib/utils'

export type EditorLanguage =
  | 'json'
  | 'yaml'
  | 'xml'
  | 'html'
  | 'css'
  | 'javascript'
  | 'typescript'
  | 'markdown'
  | 'plaintext'
  | 'graphql'
  | 'shell'

export interface MonacoEditorProps {
  value: string
  onChange?: (value: string) => void
  language?: EditorLanguage
  readOnly?: boolean
  height?: string | number
  className?: string
  placeholder?: string
  onMount?: () => void
}

/**
 * Monaco editor wrapper.
 *
 * Uses dynamic import to keep the Monaco bundle out of the initial load.
 * Falls back to a styled `<textarea>` if Monaco is not available
 * (e.g. when @monaco-editor/react is not installed yet).
 */
export const MonacoEditor = React.forwardRef<HTMLDivElement, MonacoEditorProps>(
  (
    {
      value,
      onChange,
      language = 'plaintext',
      readOnly = false,
      height = '300px',
      className,
      placeholder,
      onMount,
    },
    ref
  ) => {
    const [Editor, setEditor] = React.useState<React.ComponentType<unknown> | null>(null)
    const [loadError, setLoadError] = React.useState(false)

    React.useEffect(() => {
      let cancelled = false

      // Try to dynamically import @monaco-editor/react
      import('@monaco-editor/react')
        .then((mod) => {
          if (!cancelled) {
            setEditor(() => mod.default as unknown as React.ComponentType<unknown>)
            onMount?.()
          }
        })
        .catch(() => {
          if (!cancelled) {
            setLoadError(true)
          }
        })

      return () => {
        cancelled = true
      }
    }, [onMount])

    const heightStyle = typeof height === 'number' ? `${height}px` : height

    if (loadError || !Editor) {
      // Fallback to a textarea with syntax-aware class
      return (
        <div ref={ref} className={cn('relative', className)} style={{ height: heightStyle }}>
          <textarea
            value={value}
            onChange={(e) => onChange?.(e.target.value)}
            readOnly={readOnly}
            placeholder={placeholder}
            spellCheck={false}
            className={cn(
              'w-full h-full p-3 rounded-md border font-mono text-sm resize-none',
              'bg-white dark:bg-gray-800 border-gray-300 dark:border-gray-600',
              'text-gray-900 dark:text-gray-100',
              'focus:outline-none focus:ring-2 focus:ring-blue-500',
              'placeholder:text-gray-500'
            )}
            data-testid="monaco-fallback"
          />
          {!loadError && (
            <div className="absolute bottom-1 right-1 text-[10px] text-gray-400">
              Basic editor (Monaco loading…)
            </div>
          )}
        </div>
      )
    }

    return (
      <div
        ref={ref}
        className={cn('border border-gray-300 dark:border-gray-600 rounded-md overflow-hidden', className)}
        style={{ height: heightStyle }}
        data-testid="monaco-editor"
      >
        <Editor
          // The shape mirrors @monaco-editor/react's props
          {...({
            value,
            language,
            onChange,
            options: {
              readOnly,
              minimap: { enabled: false },
              scrollBeyondLastLine: false,
              fontSize: 13,
              fontFamily: '"JetBrains Mono", Monaco, Menlo, monospace',
              tabSize: 2,
              wordWrap: 'on',
              automaticLayout: true,
              renderWhitespace: 'selection',
              scrollbar: {
                vertical: 'auto',
                horizontal: 'auto',
              },
              placeholder,
            },
          } as Record<string, unknown>)}
        />
      </div>
    )
  }
)

MonacoEditor.displayName = 'MonacoEditor'
