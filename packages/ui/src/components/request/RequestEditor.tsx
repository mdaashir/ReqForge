import * as React from 'react'
import { cn } from '../../lib/utils'
import { MethodSelector, type MethodSelectorProps } from './MethodSelector'
import { UrlBar, type UrlBarProps } from './UrlBar'
import { BodyEditor, type BodyEditorProps } from './BodyEditor'
import { KeyValueEditor, type KeyValueEditorProps } from './KeyValueEditor'
import type { Request, BodyMode, KeyValue } from '../../types'

// ── Context ──────────────────────────────────────────────

interface RequestEditorContextValue {
  request: Request
  setRequest: (updates: Partial<Request>) => void
  onSend?: () => void
  onSave?: () => void
}

const RequestEditorContext = React.createContext<RequestEditorContextValue | null>(null)

function useRequestEditor(): RequestEditorContextValue {
  const ctx = React.useContext(RequestEditorContext)
  if (!ctx) throw new Error('RequestEditor sub-components must be used inside <RequestEditor>')
  return ctx
}

// ── Root ─────────────────────────────────────────────────

export interface RequestEditorProps {
  request: Request
  setRequest: (updates: Partial<Request>) => void
  onSend?: () => void
  onSave?: () => void
  children?: React.ReactNode
  className?: string
}

function RequestEditorRoot({ request, setRequest, onSend, onSave, children, className }: RequestEditorProps) {
  return (
    <RequestEditorContext.Provider value={{ request, setRequest, onSend, onSave }}>
      <div className={cn('flex flex-col gap-4 p-4', className)} data-testid="request-editor">
        {children}
      </div>
    </RequestEditorContext.Provider>
  )
}

// ── Sub-components ───────────────────────────────────────

function Method(props: Omit<MethodSelectorProps, 'value' | 'onChange'>) {
  const { request, setRequest } = useRequestEditor()
  return (
    <MethodSelector
      value={request.method}
      onChange={(method) => setRequest({ method })}
      {...props}
    />
  )
}

function Url(props: Omit<UrlBarProps, 'value' | 'onChange'>) {
  const { request, setRequest } = useRequestEditor()
  return (
    <UrlBar
      value={request.url}
      onChange={(url) => setRequest({ url })}
      {...props}
    />
  )
}

function Headers(props: Omit<KeyValueEditorProps, 'items' | 'onChange' | 'name'>) {
  const { request, setRequest } = useRequestEditor()
  return (
    <KeyValueEditor
      name="Headers"
      items={request.headers}
      onChange={(headers: KeyValue[]) => setRequest({ headers })}
      {...props}
    />
  )
}

function Params(props: Omit<KeyValueEditorProps, 'items' | 'onChange' | 'name'>) {
  const { request, setRequest } = useRequestEditor()
  return (
    <KeyValueEditor
      name="Query Params"
      items={request.params}
      onChange={(params: KeyValue[]) => setRequest({ params })}
      {...props}
    />
  )
}

function Body(props: Omit<BodyEditorProps, 'body' | 'onChange'>) {
  const { request, setRequest } = useRequestEditor()
  return (
    <BodyEditor
      body={request.body}
      onChange={(body) => setRequest({ body })}
      {...props}
    />
  )
}

function Send(_props: { children?: React.ReactNode; className?: string }) {
  const { onSend } = useRequestEditor()
  return onSend ? (
    <button
      onClick={onSend}
      className="inline-flex items-center justify-center h-9 px-4 rounded-md bg-blue-600 text-white text-sm font-medium hover:bg-blue-700 transition-colors"
      data-testid="request-editor-send"
    >
      Send
    </button>
  ) : null
}

function Save(_props: { children?: React.ReactNode; className?: string }) {
  const { onSave } = useRequestEditor()
  return onSave ? (
    <button
      onClick={onSave}
      className="inline-flex items-center justify-center h-9 px-4 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-800 text-sm text-gray-700 dark:text-gray-200 hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
      data-testid="request-editor-save"
    >
      Save
    </button>
  ) : null
}

function Actions({ children, className }: { children?: React.ReactNode; className?: string }) {
  const { onSend, onSave } = useRequestEditor()
  return (
    <div className={cn('flex items-center gap-2', className)} data-testid="request-editor-actions">
      {children}
      {onSave && <Save />}
      {onSend && <Send />}
    </div>
  )
}

// ── Tabs container ───────────────────────────────────────

interface Tab {
  id: string
  label: string
  content: React.ReactNode
}

function Tabs({ tabs, className }: { tabs: Tab[]; className?: string }) {
  const [active, setActive] = React.useState(tabs[0]?.id ?? '')

  // Reset to first tab when tabs change
  React.useEffect(() => {
    if (!tabs.find((t) => t.id === active)) {
      setActive(tabs[0]?.id ?? '')
    }
  }, [tabs, active])

  return (
    <div className={className}>
      <div className="flex border-b border-gray-200 dark:border-gray-700" data-testid="request-editor-tabs">
        {tabs.map((tab) => (
          <button
            key={tab.id}
            onClick={() => setActive(tab.id)}
            className={cn(
              'px-3 py-2 text-xs font-medium border-b-2 transition-colors',
              active === tab.id
                ? 'border-blue-500 text-blue-600 dark:text-blue-400'
                : 'border-transparent text-gray-500 hover:text-gray-700 dark:hover:text-gray-300'
            )}
            data-testid={`editor-tab-${tab.id}`}
          >
            {tab.label}
          </button>
        ))}
      </div>
      <div className="pt-3">
        {tabs.find((t) => t.id === active)?.content}
      </div>
    </div>
  )
}

// ── Export compound component ────────────────────────────

export const RequestEditor = Object.assign(RequestEditorRoot, {
  Method,
  Url,
  Headers,
  Params,
  Body,
  Actions,
  Tabs,
  Send,
  Save,
  useRequestEditor,
})
