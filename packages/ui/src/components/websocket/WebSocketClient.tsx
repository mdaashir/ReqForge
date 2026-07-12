import * as React from 'react'
import { Send, Plug, PlugZap, Trash2 } from 'lucide-react'
import { Button } from '../primitives/Button'
import { Input } from '../primitives/Input'
import { cn } from '../../lib/utils'

export type ConnectionState =
  | 'disconnected'
  | 'connecting'
  | 'connected'
  | 'closing'
  | 'closed'
  | 'error'

export interface WSMessage {
  id: string
  direction: 'sent' | 'received'
  type: 'text' | 'binary'
  data: string
  timestamp: number
}

export interface WebSocketClientProps {
  url: string
  onUrlChange: (url: string) => void
  onConnect: () => void
  onDisconnect: () => void
  onSend: (text: string) => void
  onClearMessages: () => void
  state: ConnectionState
  messages: WSMessage[]
  className?: string
}

const stateBadgeStyles: Record<ConnectionState, string> = {
  disconnected: 'bg-gray-100 text-gray-700 dark:bg-gray-800 dark:text-gray-300',
  connecting: 'bg-yellow-100 text-yellow-700 dark:bg-yellow-900/30 dark:text-yellow-400',
  connected: 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400',
  closing: 'bg-orange-100 text-orange-700 dark:bg-orange-900/30 dark:text-orange-400',
  closed: 'bg-gray-100 text-gray-700 dark:bg-gray-800 dark:text-gray-300',
  error: 'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400',
}

export const WebSocketClient = React.forwardRef<HTMLDivElement, WebSocketClientProps>(
  (
    {
      url,
      onUrlChange,
      onConnect,
      onDisconnect,
      onSend,
      onClearMessages,
      state,
      messages,
      className,
    },
    ref
  ) => {
    const [message, setMessage] = React.useState('')
    const messagesEndRef = React.useRef<HTMLDivElement>(null)

    React.useEffect(() => {
      messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' })
    }, [messages])

    const handleSend = () => {
      if (message.trim() && state === 'connected') {
        onSend(message)
        setMessage('')
      }
    }

    const isConnected = state === 'connected'
    const isConnecting = state === 'connecting'

    return (
      <div
        ref={ref}
        className={cn('flex flex-col h-full p-3 gap-3', className)}
        data-testid="websocket-client"
      >
        <div className="flex items-center gap-2">
          <Input
            type="url"
            value={url}
            onChange={(e) => onUrlChange(e.target.value)}
            placeholder="wss://echo.websocket.org"
            className="flex-1 font-mono"
            disabled={isConnected || isConnecting}
            aria-label="WebSocket URL"
          />
          {isConnected ? (
            <Button variant="destructive" onClick={onDisconnect}>
              <PlugZap className="h-4 w-4" />
              Disconnect
            </Button>
          ) : (
            <Button onClick={onConnect} loading={isConnecting} disabled={!url}>
              <Plug className="h-4 w-4" />
              Connect
            </Button>
          )}
        </div>

        <div className="flex items-center gap-2 text-xs">
          <span className="font-semibold text-gray-700 dark:text-gray-300">Status:</span>
          <span
            className={cn(
              'px-2 py-0.5 rounded font-semibold uppercase',
              stateBadgeStyles[state]
            )}
            data-testid="ws-state"
          >
            {state}
          </span>
          <span className="text-gray-500 dark:text-gray-400 ml-auto">
            {messages.length} {messages.length === 1 ? 'message' : 'messages'}
          </span>
        </div>

        <div
          className="flex-1 min-h-0 overflow-auto p-2 rounded-md border border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-900"
          data-testid="ws-messages"
        >
          {messages.length === 0 ? (
            <div className="text-sm text-gray-500 dark:text-gray-400 text-center py-8">
              No messages yet. Connect to a WebSocket endpoint to start.
            </div>
          ) : (
            <div className="flex flex-col gap-1">
              {messages.map((msg) => (
                <div
                  key={msg.id}
                  className={cn(
                    'px-2 py-1 rounded text-xs font-mono break-all',
                    msg.direction === 'sent'
                      ? 'bg-blue-100 dark:bg-blue-900/30 text-blue-900 dark:text-blue-200 ml-8'
                      : 'bg-gray-100 dark:bg-gray-800 text-gray-900 dark:text-gray-100 mr-8'
                  )}
                >
                  <div className="flex items-center justify-between mb-0.5 text-[10px] opacity-70">
                    <span className="font-semibold uppercase">
                      {msg.direction} · {msg.type}
                    </span>
                    <span>{new Date(msg.timestamp).toLocaleTimeString()}</span>
                  </div>
                  <div className="whitespace-pre-wrap">{msg.data}</div>
                </div>
              ))}
              <div ref={messagesEndRef} />
            </div>
          )}
        </div>

        <div className="flex items-center gap-2">
          <Input
            value={message}
            onChange={(e) => setMessage(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter' && !e.shiftKey) {
                e.preventDefault()
                handleSend()
              }
            }}
            placeholder="Type a message..."
            className="flex-1"
            disabled={!isConnected}
            aria-label="WebSocket message"
            data-testid="ws-message-input"
          />
          <Button
            onClick={handleSend}
            disabled={!isConnected || !message.trim()}
            size="icon"
            data-testid="ws-send-button"
          >
            <Send className="h-4 w-4" />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            onClick={onClearMessages}
            disabled={messages.length === 0}
            title="Clear messages"
          >
            <Trash2 className="h-4 w-4" />
          </Button>
        </div>
      </div>
    )
  }
)

WebSocketClient.displayName = 'WebSocketClient'
