import * as React from 'react'
import { Shield, ShieldOff, Copy, Check } from 'lucide-react'
import { cn } from '../../lib/utils'
import { Button } from '../primitives/Button'

export interface JwtInspectorProps {
  /** Raw JWT string (three dot-separated segments) */
  token: string
  className?: string
}

interface ParsedJwt {
  raw: string
  header: Record<string, unknown> | null
  payload: Record<string, unknown> | null
  signature: string
  error?: string
}

function parseJwt(token: string): ParsedJwt {
  const parts = token.trim().split('.')
  if (parts.length !== 3) {
    return { raw: token, header: null, payload: null, signature: '', error: 'Not a valid JWT (expected 3 dot-separated segments)' }
  }

  try {
    const header = JSON.parse(atob(parts[0]!))
    const payload = JSON.parse(atob(parts[1]!))
    return { raw: token, header, payload, signature: parts[2]!.slice(0, 20) + '…' }
  } catch (err) {
    return { raw: token, header: null, payload: null, signature: '', error: `Parse error: ${err instanceof Error ? err.message : String(err)}` }
  }
}

function formatValue(v: unknown): string {
  if (v === null) return 'null'
  if (typeof v === 'object') return JSON.stringify(v, null, 2)
  return String(v)
}

function isExpired(exp: unknown): boolean | null {
  if (typeof exp !== 'number') return null
  return Date.now() / 1000 > exp
}

/** JWT inspector: decodes and displays the header + payload of any JWT. */
export const JwtInspector = React.forwardRef<HTMLDivElement, JwtInspectorProps>(
  ({ token, className }, ref) => {
    const parsed = React.useMemo(() => parseJwt(token), [token])
    const [copied, setCopied] = React.useState(false)

    const handleCopy = async () => {
      try {
        await navigator.clipboard.writeText(token)
        setCopied(true)
        setTimeout(() => setCopied(false), 1500)
      } catch {}
    }

    if (!token.trim()) {
      return (
        <div ref={ref} className={cn('flex flex-col items-center justify-center p-8 text-gray-500 dark:text-gray-400', className)} data-testid="jwt-inspector">
          <ShieldOff className="h-8 w-8 mb-2" />
          <p className="text-sm">No JWT to inspect. Set auth type to <strong>Bearer</strong> or <strong>JWT</strong> and provide a token.</p>
        </div>
      )
    }

    const expired = parsed.payload ? isExpired(parsed.payload.exp) : null

    return (
      <div ref={ref} className={cn('flex flex-col gap-3 p-4 bg-white dark:bg-gray-900 rounded-lg border border-gray-200 dark:border-gray-700', className)} data-testid="jwt-inspector">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Shield className={cn('h-4 w-4', expired === true ? 'text-red-500' : expired === false ? 'text-green-500' : 'text-blue-500')} />
            <h3 className="font-semibold text-sm text-gray-900 dark:text-gray-100">JWT Inspector</h3>
          </div>
          <Button variant="ghost" size="icon" onClick={handleCopy} title="Copy raw JWT">
            {copied ? <Check className="h-3 w-3 text-green-500" /> : <Copy className="h-3 w-3" />}
          </Button>
        </div>

        {parsed.error && (
          <div className="p-2 rounded bg-red-50 dark:bg-red-900/20 text-red-600 dark:text-red-400 text-xs font-mono">{parsed.error}</div>
        )}

        {expired === true && (
          <div className="p-2 rounded bg-red-50 dark:bg-red-900/20 text-red-600 dark:text-red-400 text-xs flex items-center gap-1">
            <ShieldOff className="h-3 w-3" /> Token expired at {new Date((parsed.payload!.exp as number) * 1000).toLocaleString()}
          </div>
        )}
        {expired === false && (
          <div className="p-2 rounded bg-green-50 dark:bg-green-900/20 text-green-600 dark:text-green-400 text-xs flex items-center gap-1">
            <Shield className="h-3 w-3" /> Token valid until {new Date((parsed.payload!.exp as number) * 1000).toLocaleString()}
          </div>
        )}

        {parsed.header && <Section title="Header" data={parsed.header} />}
        {parsed.payload && <Section title="Payload" data={parsed.payload} />}

        <div className="flex flex-col gap-1">
          <label className="text-[10px] font-semibold uppercase text-gray-500 dark:text-gray-400 tracking-wider">Signature</label>
          <code className="text-xs font-mono text-gray-600 dark:text-gray-400 truncate">{parsed.signature}</code>
        </div>
      </div>
    )
  }
)

JwtInspector.displayName = 'JwtInspector'

const Section: React.FC<{ title: string; data: Record<string, unknown> }> = ({ title, data }) => (
  <div className="flex flex-col gap-1">
    <label className="text-[10px] font-semibold uppercase text-gray-500 dark:text-gray-400 tracking-wider">{title} ({Object.keys(data).length})</label>
    <div className="max-h-48 overflow-y-auto rounded border border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800/50 p-2 font-mono text-xs">
      {Object.entries(data).map(([key, value]) => (
        <div key={key} className="flex gap-2 py-0.5">
          <span className="text-blue-600 dark:text-blue-400 font-semibold shrink-0">{key}:</span>
          <span className="text-gray-800 dark:text-gray-200 break-all whitespace-pre-wrap">{formatValue(value)}</span>
        </div>
      ))}
    </div>
  </div>
)
