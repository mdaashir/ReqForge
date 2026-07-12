import * as React from 'react'
import { Moon, Sun, Monitor } from 'lucide-react'
import { useUIStore } from '../../stores/uiStore'
import { cn } from '../../lib/utils'
import type { Theme } from '../../types'

export interface ThemeToggleProps {
  className?: string
  /** Show a labelled cycle button instead of a single icon button */
  variant?: 'icon' | 'cycle'
}

const order: Theme[] = ['light', 'dark', 'system']

export const ThemeToggle = React.forwardRef<HTMLButtonElement, ThemeToggleProps>(
  ({ className, variant = 'icon' }, ref) => {
    const { theme, setTheme } = useUIStore()

    const cycle = () => {
      const idx = order.indexOf(theme || 'system')
      const next: Theme = order[(idx + 1) % order.length] ?? order[0] ?? 'system'
      setTheme(next)
    }

    const Icon =
      theme === 'light' ? Sun : theme === 'dark' ? Moon : Monitor

    return (
      <button
        ref={ref}
        onClick={cycle}
        title={`Theme: ${theme} (click to cycle)`}
        aria-label={`Theme: ${theme}`}
        className={cn(
          'inline-flex items-center justify-center gap-2',
          variant === 'icon' ? 'h-8 w-8 rounded-md hover:bg-gray-100 dark:hover:bg-gray-800' : '',
          'text-gray-700 dark:text-gray-300',
          'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500',
          'transition-colors',
          className
        )}
        data-testid="theme-toggle"
      >
        <Icon className="h-4 w-4" />
        {variant === 'cycle' && (
          <span className="text-xs capitalize">{theme}</span>
        )}
      </button>
    )
  }
)

ThemeToggle.displayName = 'ThemeToggle'
