import { useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import {
  UrlBar,
  ResponseViewer,
  Tabs,
  TabsList,
  TabsTrigger,
  TabsContent,
  CollectionTree,
  EnvironmentSelector,
  AuthEditor,
  KeyValueEditor,
  BodyEditor,
  TestEditor,
  GrpcEditor,
  useRequest,
  useEnvironmentStore,
  useUIStore,
  useKeyboard,
  type AuthConfig,
  type KeyValue,
  type RequestBody,
  type GrpcMethod,
} from '@reqforge/ui'
import './App.css'

interface AppInfo {
  name: string
  version: string
}

function App() {
  const {
    request,
    response,
    loading,
    error,
    sendRequest,
    updateRequest,
    undo,
    redo,
  } = useRequest()
  const { sidebarOpen } = useUIStore()
  const {
    environments,
    activeEnvironmentId,
    addEnvironment,
    setActiveEnvironment,
  } = useEnvironmentStore()

  const [appInfo, setAppInfo] = useState<AppInfo | null>(null)
  const [grpcMode, setGrpcMode] = useState(false)
  const [grpcMethods, setGrpcMethods] = useState<GrpcMethod[]>([])

  useEffect(() => {
    const loadAppInfo = async () => {
      try {
        const [name, version] = await Promise.all([
          invoke<string>('get_app_name'),
          invoke<string>('get_app_version'),
        ])
        setAppInfo({ name, version })
      } catch (err) {
        console.error('Failed to load app info:', err)
      }
    }
    loadAppInfo()
  }, [])

  // Undo/redo keyboard shortcuts.
  useKeyboard(
    [
      {
        key: 'z',
        ctrl: true,
        shift: false,
        handler: () => undo(),
      },
      {
        key: 'z',
        ctrl: true,
        shift: true,
        handler: () => redo(),
      },
    ],
    true
  )

  // Mock collections for demo - in production these come from Tauri
  const mockCollections = [
    {
      id: 'demo-1',
      name: 'Demo API',
      description: 'Example collection',
      headers: [],
      variables: [],
      items: [
        {
          id: 'req-1',
          type: 'request' as const,
          name: 'Get Users',
          method: 'GET' as const,
          url: 'https://jsonplaceholder.typicode.com/users',
          headers: [],
          params: [],
          body: { mode: 'none' as const, content: '' },
          followRedirects: true,
          verifySSL: true,
        },
        {
          id: 'req-2',
          type: 'request' as const,
          name: 'Get Posts',
          method: 'GET' as const,
          url: 'https://jsonplaceholder.typicode.com/posts',
          headers: [],
          params: [],
          body: { mode: 'none' as const, content: '' },
          followRedirects: true,
          verifySSL: true,
        },
      ],
      createdAt: Date.now(),
      updatedAt: Date.now(),
    },
  ]

  const handleAuthChange = (auth: AuthConfig | undefined) => {
    updateRequest({ auth })
  }

  return (
    <div className="app">
      <header className="app-header">
        <h1>{appInfo?.name || 'ReqForge'}</h1>
        <span className="version">v{appInfo?.version || '0.1.0'}</span>
        <div className="ml-auto">
          <EnvironmentSelector
            environments={environments}
            activeEnvironmentId={activeEnvironmentId}
            onSelectEnvironment={setActiveEnvironment}
            onCreateEnvironment={(name: string) =>
              addEnvironment({
                id: crypto.randomUUID(),
                name,
                variables: [],
                created_at: new Date().toISOString(),
                updated_at: new Date().toISOString(),
              })
            }
          />
        </div>
      </header>

      <div className="app-body">
        {sidebarOpen && (
          <aside className="app-sidebar">
            <CollectionTree
              collections={mockCollections}
              onSelectRequest={(collectionId: string, requestId: string) => {
                const collection = mockCollections.find((c) => c.id === collectionId)
                const item = collection?.items.find((i) => i.id === requestId)
                if (item && item.type === 'request') {
                  updateRequest({
                    id: item.id,
                    name: item.name,
                    method: item.method,
                    url: item.url,
                    headers: item.headers,
                    params: item.params,
                    body: item.body,
                  })
                }
              }}
              onReorder={(collectionId: string, newItems) => {
                // Update local mock state and persist.
                const c = mockCollections.find((c) => c.id === collectionId) as
                  | { items: typeof newItems; id: string; name: string; description: string; auth: unknown; headers: unknown; variables: unknown }
                  | undefined
                if (c) (c as any).items = newItems
                // Fire-and-forget persist to disk.
                invoke('save_collection', { collection: c, save: true }).catch(
                  (err) => console.error('Failed to save reordered collection:', err)
                )
              }}
            />
          </aside>
        )}

        <main className="app-main">
          <div className="flex items-center justify-end mb-2">
            <button
              onClick={() => setGrpcMode(!grpcMode)}
              className="text-xs px-2 py-1 rounded text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800"
              data-testid="toggle-grpc"
            >
              {grpcMode ? 'Switch to REST' : 'Switch to gRPC'}
            </button>
          </div>

          {grpcMode ? (
            <GrpcEditor
              endpoint={request.url}
              selectedMethod={undefined}
              body={request.body.content}
              headers={request.headers}
              methods={grpcMethods}
              loading={loading}
              onEndpointChange={(url: string) => updateRequest({ url })}
              onMethodSelect={(method: string) => updateRequest({ url: method })}
              onBodyChange={(body: string) => updateRequest({ body: { ...request.body, content: body } })}
              onHeadersChange={(headers: KeyValue[]) => updateRequest({ headers })}
              onProtoLoaded={setGrpcMethods}
              onSend={sendRequest}
              className="mb-3"
            />
          ) : (
          <>
          <UrlBar
            method={request.method}
            url={request.url}
            onMethodChange={(method: string) => updateRequest({ method })}
            onUrlChange={(url: string) => updateRequest({ url })}
            onSend={sendRequest}
            loading={loading}
            className="mb-3"
          />

          <Tabs defaultValue="params">
            <TabsList>
              <TabsTrigger value="params">Params</TabsTrigger>
              <TabsTrigger value="headers">Headers</TabsTrigger>
              <TabsTrigger value="body">Body</TabsTrigger>
              <TabsTrigger value="auth">Auth</TabsTrigger>
              <TabsTrigger value="tests">Tests</TabsTrigger>
            </TabsList>

            <TabsContent value="params">
              <KeyValueEditor
                items={request.params}
                onChange={(params: KeyValue[]) => updateRequest({ params })}
                keyPlaceholder="Parameter name"
                valuePlaceholder="Value"
              />
            </TabsContent>

            <TabsContent value="headers">
              <KeyValueEditor
                items={request.headers}
                onChange={(headers: KeyValue[]) => updateRequest({ headers })}
                keyPlaceholder="Header name"
                valuePlaceholder="Header value"
              />
            </TabsContent>

            <TabsContent value="body">
              <BodyEditor
                body={request.body}
                onChange={(body: RequestBody) => updateRequest({ body })}
              />
            </TabsContent>

            <TabsContent value="auth">
              <AuthEditor auth={request.auth} onChange={handleAuthChange} />
            </TabsContent>

            <TabsContent value="tests">
              <TestEditor response={response} />
            </TabsContent>
          </Tabs>
          </>)}

          <div className="mt-4">
            <ResponseViewer response={response} loading={loading} error={error} />
          </div>
        </main>
      </div>
    </div>
  )
}

export default App
