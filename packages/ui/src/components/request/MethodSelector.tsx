import * as React from 'react'
import { Select, type SelectOption } from '../primitives/Select'
import { cn } from '../../lib/utils'
import type { HttpMethod } from '../../types'

const METHOD_OPTIONS: SelectOption[] = [
  { value: 'GET', label: 'GET' },
  { value: 'POST', label: 'POST' },
  { value: 'PUT', label: 'PUT' },
  { value: 'PATCH', label: 'PATCH' },
  { value: 'DELETE', label: 'DELETE' },
  { value: 'HEAD', label: 'HEAD' },
  { value: 'OPTIONS', label: 'OPTIONS' },
  { value: 'TRACE', label: 'TRACE' },
  { value: 'CONNECT', label: 'CONNECT' },
]

const methodColors: Record<string, string> = {
  GET: 'text-green-600 dark:text-green-400',
  POST: 'text-orange-600 dark:text-orange-400',
  PUT: 'text-blue-600 dark:text-blue-400',
  PATCH: 'text-purple-600 dark:text-purple-400',
  DELETE: 'text-red-600 dark:text-red-400',
  HEAD: 'text-gray-600 dark:text-gray-400',
  OPTIONS: 'text-gray-600 dark:text-gray-400',
  TRACE: 'text-gray-600 dark:text-gray-400',
  CONNECT: 'text-gray-600 dark:text-gray-400',
}

export interface MethodSelectorProps {
  value: HttpMethod
  onChange: (value: HttpMethod) => void
  className?: string
}

export const MethodSelector = React.forwardRef<HTMLSelectElement, MethodSelectorProps>(
  ({ value, onChange, className }, ref) => {
    const colorClass = methodColors[value] || 'text-gray-600 dark:text-gray-400'

    return (
      <Select
        ref={ref}
        value={value}
        onChange={(e) => onChange(e.target.value as HttpMethod)}
        options={METHOD_OPTIONS}
        className={cn('w-28 font-mono font-semibold', colorClass, className)}
        aria-label="HTTP method"
      />
    )
  }
)

MethodSelector.displayName = 'MethodSelector'
