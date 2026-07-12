import * as React from 'react'
import { Send } from 'lucide-react'
import { Button } from '../primitives/Button'
import { Input } from '../primitives/Input'
import { MethodSelector } from './MethodSelector'
import { cn } from '../../lib/utils'
import type { HttpMethod } from '../../types'

export interface UrlBarProps {
  method: HttpMethod
  url: string
  onMethodChange: (method: HttpMethod) => void
  onUrlChange: (url: string) => void
  onSend: () => void
  loading?: boolean
  className?: string
}

export const UrlBar = React.forwardRef<HTMLDivElement, UrlBarProps>(
  (
    { method, url, onMethodChange, onUrlChange, onSend, loading, className },
    ref
  ) => {
    return (
      <div
        ref={ref}
        className={cn('flex items-center gap-2', className)}
        data-testid="url-bar"
      >
        <MethodSelector value={method} onChange={onMethodChange} />

        <Input
          type="url"
          value={url}
          onChange={(e) => onUrlChange(e.target.value)}
          placeholder="Enter request URL (e.g., https://api.example.com/users)"
          className="flex-1 font-mono"
          data-testid="url-input"
          aria-label="Request URL"
          onKeyDown={(e) => {
            if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
              e.preventDefault()
              onSend()
            }
          }}
        />

        <Button
          onClick={onSend}
          loading={loading}
          disabled={!url}
          data-testid="send-button"
        >
          <Send className="h-4 w-4" />
          Send
        </Button>
      </div>
    )
  }
)

UrlBar.displayName = 'UrlBar'
