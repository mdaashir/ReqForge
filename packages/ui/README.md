# @reqforge/ui

Shared React component library for ReqForge. Built with TypeScript, shadcn/ui-style primitives, Tailwind CSS, and Zustand state management.

## Installation

```bash
pnpm add @reqforge/ui
```

## Architecture

```
components/
├── auth/          AuthEditor, JwtInspector
├── collab/        CollabCursors (real-time presence)
├── collection/    CollectionTree (drag-and-drop + virtualized)
├── dialogs/       CommandPalette, ImportDialog
├── environment/   EnvironmentSelector, EnvironmentManager
├── graphql/       GraphQLEditor
├── grpc/          GrpcEditor
├── history/       HistoryViewer
├── layout/        AppShell, Sidebar, TabBar, PanelLayout, StatusBar, ActivityBar
├── onboarding/    OnboardingTour
├── plugins/       PluginManager
├── primitives/    Button, Input, Select, Tabs, MonacoEditor
├── request/       RequestEditor (compound), UrlBar, MethodSelector, BodyEditor, KeyValueEditor, CodeSnippetViewer
├── response/      ResponseViewer, BodyViewer, HeadersViewer, TimingViewer
├── testing/       TestEditor, TestResults
└── websocket/     WebSocketClient

hooks/
├── useAuth, useAwareness, useCollection, useCommandPaletteActions
├── useEnvironment, useJsonSchema, useKeyboard, useKeychain
├── useOAuthFlow, usePlugins, useRequest, useTestRunner, useTheme

stores/ (Zustand)
├── collectionStore, environmentStore, historyStore, requestStore
├── settingsStore, tabStore, uiStore
```

## Quick example

```tsx
import { AppShell, RequestEditor, ResponseViewer, useRequest } from '@reqforge/ui'

function App() {
  const { request, response, loading, sendRequest, updateRequest } = useRequest()

  return (
    <AppShell>
      <RequestEditor
        request={request}
        setRequest={updateRequest}
        onSend={sendRequest}
      >
        <div className="flex gap-2">
          <RequestEditor.Method />
          <RequestEditor.Url />
          <RequestEditor.Send />
        </div>
        <RequestEditor.Tabs
          tabs={[
            { id: 'body', label: 'Body', content: <RequestEditor.Body /> },
            { id: 'headers', label: 'Headers', content: <RequestEditor.Headers /> },
            { id: 'params', label: 'Params', content: <RequestEditor.Params /> },
          ]}
        />
      </RequestEditor>
      <ResponseViewer response={response} loading={loading} />
    </AppShell>
  )
}
```

## Compound components

`RequestEditor` uses the compound component pattern with shared context:

| Sub-component | Description |
|--------------|-------------|
| `<RequestEditor.Method />` | HTTP method selector dropdown |
| `<RequestEditor.Url />` | URL input with placeholder |
| `<RequestEditor.Headers />` | Key-value editor for headers |
| `<RequestEditor.Params />` | Key-value editor for query params |
| `<RequestEditor.Body />` | Body editor with mode selector and Monaco |
| `<RequestEditor.Tabs />` | Tabbed sub-view |
| `<RequestEditor.Actions />` | Action bar with Send + Save buttons |
| `<RequestEditor.Send />` | Standalone send button |
| `<RequestEditor.Save />` | Standalone save button |

## All hooks

| Hook | Returns | Purpose |
|------|---------|---------|
| `useRequest()` | `{ request, response, loading, error, sendRequest, ... }` | Send requests via Tauri IPC |
| `useCollection()` | `{ collections, activeId, create, delete, ... }` | Collection CRUD operations |
| `useAuth()` | `{ getToken, storeCredential, ... }` | Auth token + credential management |
| `useTheme()` | `{ theme, resolved, setTheme, toggle }` | Light/dark/system theme with persist |
| `useKeyboard()` | `{ register, unregister }` | Keyboard shortcut bindings |
| `useEnvironment()` | `{ environments, activeEnv, variables }` | Environment variable management |
| `usePlugins()` | `{ plugins, install, uninstall }` | Plugin lifecycle management |
| `useTestRunner()` | `{ run, results, summary }` | Execute test suites |
| `useAwareness()` | `{ cursors, update }` | Collaborative cursor state |
| `useOAuthFlow()` | `{ start, exchange, token }` | OAuth 2.0 PKCE browser flow |
| `useKeychain()` | `{ get, set, delete, list }` | OS keychain access |
| `useJsonSchema()` | `{ valid, errors }` | JSON Schema draft-07 validation |
| `useCommandPaletteActions()` | `{ registerAction, execute }` | Command palette action registry |

## All stores

| Store | Key fields | Purpose |
|-------|-----------|---------|
| `useRequestStore` | `request, response, loading, past, future` | Request state with undo/redo |
| `useCollectionStore` | `collections, activeId, loading` | Collection tree state |
| `useEnvironmentStore` | `environments, activeEnv, variables` | Environment management |
| `useHistoryStore` | `items, loading, searchQuery` | Request history with search |
| `useSettingsStore` | `settings, update` | User preferences |
| `useTabStore` | `tabs, activeId` | Open tabs with reorder |
| `useUIStore` | `theme, sidebarOpen, paletteOpen, panelHeight` | App chrome state |

## Styling

All components use Tailwind CSS with dark mode via `class` strategy. The theme toggle adds/removes the `dark` class on `document.documentElement`.

```tsx
// Your tailwind.config should reference this package:
module.exports = {
  presets: [require('@reqforge/ui/tailwind.config')],
  darkMode: 'class',
}
```

## Development

```bash
pnpm --filter @reqforge/ui typecheck  # TypeScript check
pnpm --filter @reqforge/ui lint       # ESLint
```
