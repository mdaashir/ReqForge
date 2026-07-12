import * as React from 'react'
import { Send, FileCode, FileJson, Upload, ChevronRight, Server, Braces } from 'lucide-react'
import { open } from '@tauri-apps/plugin-dialog'
import { Button } from '../primitives/Button'
import { Input } from '../primitives/Input'
import { Tabs, TabsList, TabsTrigger, TabsContent } from '../primitives/Tabs'
import { cn } from '../../lib/utils'

export interface GrpcMethod {
  service: string
  method: string
  request_type: string
  response_type: string
  client_streaming: boolean
  server_streaming: boolean
}

export interface GrpcEditorProps {
  /** gRPC-Web JSON endpoint, e.g. https://api.example.com/helloworld.Greeter/SayHello */
  endpoint: string
  /** Currently selected method (service/name) */
  selectedMethod?: string
  /** Request body as JSON */
  body: string
  /** Headers */
  headers: Array<{ key: string; value: string; enabled: boolean }>
  /** Methods discovered from the .proto file */
  methods: GrpcMethod[]
  loading?: boolean
  onEndpointChange: (endpoint: string) => void
  onMethodSelect: (method: string) => void
  onBodyChange: (body: string) => void
  onHeadersChange: (headers: Array<{ key: string; value: string; enabled: boolean }>) => void
  /** Called when the user pastes or loads a .proto file */
  onProtoLoaded: (methods: GrpcMethod[]) => void
  onSend: () => void
  className?: string
}

/** Parse a minimal .proto file and extract service/method definitions. */
function parseProtoMethods(text: string): GrpcMethod[] {
  const methods: GrpcMethod[] = []
  const serviceRegex = /service\s+(\w+)\s*\{([\s\S]*?)\}/g
  const methodRegex = /rpc\s+(\w+)\s*\(\s*(stream\s+)?(\w+)\s*\)\s*returns\s*\(\s*(stream\s+)?(\w+)\s*\)/g
  let serviceMatch: RegExpExecArray | null
  while ((serviceMatch = serviceRegex.exec(text)) !== null) {
    const serviceName = serviceMatch[1]
    const serviceBody = serviceMatch[2]
    if (!serviceName || !serviceBody) continue
    let methodMatch: RegExpExecArray | null
    const localMethodRegex = new RegExp(methodRegex.source, 'g')
    while ((methodMatch = localMethodRegex.exec(serviceBody)) !== null) {
      methods.push({
        service: serviceName,
        method: methodMatch[1] || '',
        request_type: methodMatch[3] || '',
        response_type: methodMatch[5] || '',
        client_streaming: !!methodMatch[2],
        server_streaming: !!methodMatch[4],
      })
    }
  }
  return methods
}

export const GrpcEditor = React.forwardRef<HTMLDivElement, GrpcEditorProps>(
  (
    {
      endpoint,
      selectedMethod,
      body,
      headers,
      methods,
      loading,
      onEndpointChange,
      onMethodSelect,
      onBodyChange,
      onHeadersChange,
      onProtoLoaded,
      onSend,
      className,
    },
    ref
  ) => {
    const [protoText, setProtoText] = React.useState('')
    const [error, setError] = React.useState<string | null>(null)
    const [sidebarOpen, setSidebarOpen] = React.useState(true)

    const bodyValid = React.useMemo(() => {
      if (!body.trim()) return true
      try {
        JSON.parse(body)
        return true
      } catch {
        return false
      }
    }, [body])

    const selectedMethodObj = React.useMemo(
      () => methods.find((m) => `${m.service}.${m.method}` === selectedMethod),
      [methods, selectedMethod]
    )

    const handleLoadFromFile = async () => {
      try {
        const path = await open({
          multiple: false,
          filters: [{ name: 'Protocol Buffers', extensions: ['proto'] }],
        })
        if (typeof path === 'string') {
          // Read the file via fetch (Tauri serves it via its asset protocol,
          // but for raw disk reads we'd use a Tauri command). For the browser
          // build we just prompt the user to paste the contents.
          // In the desktop app the user can use "Paste" instead.
          setError(
            'File picker is wired to the desktop shell. Paste the file contents below.'
          )
        }
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e))
      }
    }

    const handleParseProto = () => {
      setError(null)
      try {
        const parsed = parseProtoMethods(protoText)
        if (parsed.length === 0) {
          setError('No service definitions found in the .proto file')
          return
        }
        onProtoLoaded(parsed)
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e))
      }
    }

    // Group methods by service for the sidebar tree.
    const services = React.useMemo(() => {
      const map = new Map<string, GrpcMethod[]>()
      for (const m of methods) {
        if (!map.has(m.service)) map.set(m.service, [])
        map.get(m.service)!.push(m)
      }
      return Array.from(map.entries())
    }, [methods])

    return (
      <div ref={ref} className={cn('flex flex-col gap-3 p-3', className)} data-testid="grpc-editor">
        <div className="flex items-center gap-2">
          <Button
            variant="ghost"
            size="icon"
            onClick={() => setSidebarOpen(!sidebarOpen)}
            title="Toggle service tree"
            data-testid="grpc-toggle-sidebar"
          >
            <ChevronRight
              className={cn('h-4 w-4 transition-transform', sidebarOpen && 'rotate-90')}
            />
          </Button>
          <Input
            type="url"
            value={endpoint}
            onChange={(e) => onEndpointChange(e.target.value)}
            placeholder="https://api.example.com/helloworld.Greeter/SayHello"
            className="flex-1 font-mono"
            aria-label="gRPC-Web endpoint"
          />
          <Button onClick={onSend} loading={loading} disabled={!endpoint || !bodyValid}>
            <Send className="h-4 w-4" />
            Send
          </Button>
        </div>

        {selectedMethodObj && (
          <div className="flex items-center gap-2 text-xs text-gray-600 dark:text-gray-400">
            <Braces className="h-3 w-3" />
            <span className="font-mono">
              {selectedMethodObj.service}.{selectedMethodObj.method}
            </span>
            <span>→ {selectedMethodObj.response_type}</span>
            {selectedMethodObj.client_streaming && (
              <span className="px-1 rounded bg-orange-100 dark:bg-orange-900/30 text-orange-700 dark:text-orange-400">
                client stream
              </span>
            )}
            {selectedMethodObj.server_streaming && (
              <span className="px-1 rounded bg-purple-100 dark:bg-purple-900/30 text-purple-700 dark:text-purple-400">
                server stream
              </span>
            )}
          </div>
        )}

        {error && (
          <div className="p-2 rounded bg-red-50 dark:bg-red-900/20 text-red-600 dark:text-red-400 text-xs">
            {error}
          </div>
        )}

        <div className="flex gap-3">
          {sidebarOpen && services.length > 0 && (
            <aside className="w-56 flex-shrink-0 border border-gray-200 dark:border-gray-700 rounded-md p-2 max-h-96 overflow-y-auto">
              <h3 className="text-xs font-semibold uppercase text-gray-500 dark:text-gray-400 mb-2 px-1">
                Services
              </h3>
              {services.map(([svc, methods]) => (
                <div key={svc} className="mb-2">
                  <div className="flex items-center gap-1 px-1 py-1 text-xs font-semibold text-gray-700 dark:text-gray-300">
                    <Server className="h-3 w-3" />
                    {svc}
                  </div>
                  <div className="ml-2 flex flex-col gap-0.5">
                    {methods.map((m) => {
                      const id = `${m.service}.${m.method}`
                      if (!id) return null
                      return (
                        <button
                          key={id}
                          onClick={() => onMethodSelect(id)}
                          className={cn(
                            'text-left text-xs px-2 py-1 rounded',
                            selectedMethod === id
                              ? 'bg-blue-100 dark:bg-blue-900/40 text-blue-900 dark:text-blue-100'
                              : 'hover:bg-gray-100 dark:hover:bg-gray-800 text-gray-700 dark:text-gray-300'
                          )}
                          data-testid={`grpc-method-${id}`}
                        >
                          {m.method}
                        </button>
                      )
                    })}
                  </div>
                </div>
              ))}
            </aside>
          )}

          <div className="flex-1 min-w-0">
            <Tabs defaultValue={selectedMethod ? 'body' : 'proto'}>
              <TabsList>
                <TabsTrigger value="proto">
                  <FileCode className="h-4 w-4 mr-1" />
                  Proto
                </TabsTrigger>
                <TabsTrigger value="body" disabled={!selectedMethod}>
                  <FileJson className="h-4 w-4 mr-1" />
                  Body
                </TabsTrigger>
                <TabsTrigger value="headers">Headers</TabsTrigger>
              </TabsList>

              <TabsContent value="proto">
                <div className="flex flex-col gap-2">
                  <Button variant="outline" size="sm" onClick={handleLoadFromFile}>
                    <Upload className="h-3 w-3 mr-1" />
                    Open .proto file…
                  </Button>
                  <textarea
                    value={protoText}
                    onChange={(e) => setProtoText(e.target.value)}
                    placeholder="Paste your .proto file here, then click Parse."
                    className={cn(
                      'w-full h-48 p-3 rounded-md border border-gray-300 dark:border-gray-600',
                      'bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100',
                      'font-mono text-xs resize-y',
                      'focus:outline-none focus:ring-2 focus:ring-blue-500'
                    )}
                    spellCheck={false}
                    data-testid="grpc-proto-text"
                  />
                  <Button onClick={handleParseProto} disabled={!protoText.trim()}>
                    Parse Proto
                  </Button>
                </div>
              </TabsContent>

              <TabsContent value="body">
                <textarea
                  value={body}
                  onChange={(e) => onBodyChange(e.target.value)}
                  placeholder={selectedMethodObj ? `{}` : 'Select a method first'}
                  disabled={!selectedMethod}
                  className={cn(
                    'w-full h-64 p-3 rounded-md border font-mono text-sm resize-y',
                    'bg-white dark:bg-gray-800',
                    'focus:outline-none focus:ring-2',
                    bodyValid
                      ? 'border-gray-300 dark:border-gray-600 text-gray-900 dark:text-gray-100 focus:ring-blue-500'
                      : 'border-red-500 text-red-600 focus:ring-red-500'
                  )}
                  spellCheck={false}
                  data-testid="grpc-body"
                />
                {!bodyValid && (
                  <p className="text-xs text-red-600 dark:text-red-400 mt-1">Invalid JSON</p>
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
        </div>
      </div>
    )
  }
)

GrpcEditor.displayName = 'GrpcEditor'

// Re-export parser for testing.
export { parseProtoMethods as _parseProtoMethods }
