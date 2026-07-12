import * as React from 'react'
import { Input } from '../primitives/Input'
import { Select } from '../primitives/Select'
import { cn } from '../../lib/utils'
import type { AuthConfig, AuthType } from '../../types'

export interface AuthEditorProps {
  auth?: AuthConfig
  onChange: (auth: AuthConfig | undefined) => void
  className?: string
}

const AUTH_TYPE_OPTIONS = [
  { value: 'none', label: 'No Auth' },
  { value: 'apiKey', label: 'API Key' },
  { value: 'bearer', label: 'Bearer Token' },
  { value: 'basic', label: 'Basic Auth' },
  { value: 'oauth2', label: 'OAuth 2.0' },
  { value: 'jwt', label: 'JWT' },
]

const API_KEY_LOCATION_OPTIONS = [
  { value: 'header', label: 'In Header' },
  { value: 'query', label: 'In Query Params' },
  { value: 'cookie', label: 'In Cookie' },
]

export const AuthEditor = React.forwardRef<HTMLDivElement, AuthEditorProps>(
  ({ auth, onChange, className }, ref) => {
    const type: AuthType = auth?.type || 'none'
    const config = auth?.config || {}

    const handleTypeChange = (newType: string) => {
      if (newType === 'none') {
        onChange(undefined)
      } else {
        onChange({ type: newType as AuthType, config: {} })
      }
    }

    const updateConfig = (key: string, value: string) => {
      onChange({
        type,
        config: { ...config, [key]: value },
      })
    }

    return (
      <div ref={ref} className={cn('flex flex-col gap-3 p-3', className)} data-testid="auth-editor">
        <div>
          <label className="text-xs font-semibold text-gray-700 dark:text-gray-300 mb-1 block">
            Auth Type
          </label>
          <Select
            options={AUTH_TYPE_OPTIONS}
            value={type}
            onChange={(e) => handleTypeChange(e.target.value)}
          />
        </div>

        {type === 'apiKey' && (
          <>
            <div>
              <label className="text-xs font-semibold text-gray-700 dark:text-gray-300 mb-1 block">
                Key Name
              </label>
              <Input
                placeholder="X-API-Key"
                value={config.key || ''}
                onChange={(e) => updateConfig('key', e.target.value)}
              />
            </div>
            <div>
              <label className="text-xs font-semibold text-gray-700 dark:text-gray-300 mb-1 block">
                Value
              </label>
              <Input
                placeholder="your-api-key"
                value={config.value || ''}
                onChange={(e) => updateConfig('value', e.target.value)}
              />
            </div>
            <div>
              <label className="text-xs font-semibold text-gray-700 dark:text-gray-300 mb-1 block">
                Add to
              </label>
              <Select
                options={API_KEY_LOCATION_OPTIONS}
                value={config.location || 'header'}
                onChange={(e) => updateConfig('location', e.target.value)}
              />
            </div>
          </>
        )}

        {type === 'bearer' && (
          <div>
            <label className="text-xs font-semibold text-gray-700 dark:text-gray-300 mb-1 block">
              Token
            </label>
            <Input
              type="password"
              placeholder="your-bearer-token"
              value={config.token || ''}
              onChange={(e) => updateConfig('token', e.target.value)}
            />
          </div>
        )}

        {type === 'basic' && (
          <>
            <div>
              <label className="text-xs font-semibold text-gray-700 dark:text-gray-300 mb-1 block">
                Username
              </label>
              <Input
                placeholder="username"
                value={config.username || ''}
                onChange={(e) => updateConfig('username', e.target.value)}
              />
            </div>
            <div>
              <label className="text-xs font-semibold text-gray-700 dark:text-gray-300 mb-1 block">
                Password
              </label>
              <Input
                type="password"
                placeholder="password"
                value={config.password || ''}
                onChange={(e) => updateConfig('password', e.target.value)}
              />
            </div>
          </>
        )}

        {type === 'oauth2' && (
          <div>
            <label className="text-xs font-semibold text-gray-700 dark:text-gray-300 mb-1 block">
              Access Token
            </label>
            <Input
              type="password"
              placeholder="ya29.AccessToken"
              value={config.access_token || ''}
              onChange={(e) => updateConfig('access_token', e.target.value)}
            />
            <p className="text-xs text-gray-500 dark:text-gray-400 mt-2">
              Use the OAuth 2.0 flow in Settings to obtain a fresh token.
            </p>
          </div>
        )}

        {type === 'jwt' && (
          <div>
            <label className="text-xs font-semibold text-gray-700 dark:text-gray-300 mb-1 block">
              JWT Token
            </label>
            <Input
              type="password"
              placeholder="eyJhbGciOi..."
              value={config.token || ''}
              onChange={(e) => updateConfig('token', e.target.value)}
            />
            <p className="text-xs text-gray-500 dark:text-gray-400 mt-2">
              Token will be sent as <code>Authorization: Bearer &lt;token&gt;</code>
            </p>
          </div>
        )}
      </div>
    )
  }
)

AuthEditor.displayName = 'AuthEditor'
