import { useCallback, useEffect, useSyncExternalStore } from 'react'

type Theme = 'light' | 'dark' | 'system'

const THEME_KEY = 'reqforge-theme'

function getStoredTheme(): Theme {
  if (typeof window === 'undefined') return 'system'
  return (localStorage.getItem(THEME_KEY) as Theme) ?? 'system'
}

function setStoredTheme(t: Theme) {
  localStorage.setItem(THEME_KEY, t)
}

function resolveTheme(theme: Theme): 'light' | 'dark' {
  if (theme === 'system') {
    return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
  }
  return theme
}

function applyTheme(resolved: 'light' | 'dark') {
  document.documentElement.classList.toggle('dark', resolved === 'dark')
}

const listeners = new Set<() => void>()

function subscribe(cb: () => void) {
  listeners.add(cb)
  return () => listeners.delete(cb)
}

function getSnapshot(): Theme {
  return getStoredTheme()
}

function emitChange() {
  listeners.forEach((l) => l())
}

interface UseThemeReturn {
  theme: Theme
  resolved: 'light' | 'dark'
  setTheme: (t: Theme) => void
  toggle: () => void
}

export function useTheme(): UseThemeReturn {
  const theme = useSyncExternalStore(subscribe, getSnapshot, getStoredTheme)
  const resolved = resolveTheme(theme)

  useEffect(() => {
    applyTheme(resolved)
  }, [resolved])

  useEffect(() => {
    const mq = window.matchMedia('(prefers-color-scheme: dark)')
    const handler = () => {
      if (getStoredTheme() === 'system') {
        applyTheme(mq.matches ? 'dark' : 'light')
        emitChange()
      }
    }
    mq.addEventListener('change', handler)
    return () => mq.removeEventListener('change', handler)
  }, [])

  const setTheme = useCallback((t: Theme) => {
    setStoredTheme(t)
    applyTheme(resolveTheme(t))
    emitChange()
  }, [])

  const toggle = useCallback(() => {
    const next = resolved === 'dark' ? 'light' : 'dark'
    setTheme(next)
  }, [resolved, setTheme])

  return { theme, resolved, setTheme, toggle }
}
