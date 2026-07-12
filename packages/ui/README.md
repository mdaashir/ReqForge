# @reqforge/ui

Shared React component library for ReqForge. Used by the desktop app today, and reusable by a future web build.

## What's inside

- **Primitives** — Button, Input, Select, Tabs (shadcn/ui style with Tailwind)
- **Request** — UrlBar, MethodSelector
- **Response** — ResponseViewer (body, headers, timing)
- **Collection** — CollectionTree with nested folders
- **Environment** — EnvironmentSelector
- **Auth** — AuthEditor for all 5 auth types
- **Testing** — TestResults viewer
- **GraphQL** — GraphQLEditor
- **WebSocket** — WebSocketClient

## Hooks

- `useRequest` — Send a request through the Tauri IPC bridge to the Rust backend

## Stores (Zustand)

- `useRequestStore` — current request, response, loading, error
- `useUIStore` — theme, sidebar, command palette state
- `useEnvironmentStore` — environments, active env, variables

## Usage

```tsx
import { UrlBar, ResponseViewer, useRequest } from '@reqforge/ui'

function MyRequestView() {
  const { request, response, sendRequest, updateRequest } = useRequest()

  return (
    <>
      <UrlBar
        method={request.method}
        url={request.url}
        onMethodChange={(m) => updateRequest({ method: m })}
        onUrlChange={(u) => updateRequest({ url: u })}
        onSend={sendRequest}
      />
      <ResponseViewer response={response} />
    </>
  )
}
```

## Development

```bash
pnpm --filter @reqforge/ui typecheck
pnpm --filter @reqforge/ui lint
```
