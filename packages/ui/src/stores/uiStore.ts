import { create } from 'zustand'
import type { Theme } from '../types'

interface UIState {
  theme: Theme
  sidebarOpen: boolean
  bottomPanelOpen: boolean
  commandPaletteOpen: boolean

  setTheme: (theme: Theme) => void
  toggleTheme: () => void
  toggleSidebar: () => void
  toggleBottomPanel: () => void
  setCommandPaletteOpen: (open: boolean) => void
}

function getInitialTheme(): Theme {
  if (typeof window === 'undefined') return 'system'

  const stored = localStorage.getItem('reqforge-theme') as Theme | null
  if (stored === 'light' || stored === 'dark' || stored === 'system') {
    return stored
  }
  return 'system'
}

function applyTheme(theme: Theme) {
  if (typeof document === 'undefined') return

  const root = document.documentElement
  const isDark =
    theme === 'dark' ||
    (theme === 'system' && window.matchMedia('(prefers-color-scheme: dark)').matches)

  root.classList.toggle('dark', isDark)
  root.setAttribute('data-theme', isDark ? 'dark' : 'light')
}

export const useUIStore = create<UIState>((set, get) => ({
  theme: getInitialTheme(),
  sidebarOpen: true,
  bottomPanelOpen: true,
  commandPaletteOpen: false,

  setTheme: (theme) => {
    localStorage.setItem('reqforge-theme', theme)
    applyTheme(theme)
    set({ theme })
  },

  toggleTheme: () => {
    const current = get().theme
    const next: Theme = current === 'light' ? 'dark' : current === 'dark' ? 'system' : 'light'
    get().setTheme(next)
  },

  toggleSidebar: () => set((state) => ({ sidebarOpen: !state.sidebarOpen })),

  toggleBottomPanel: () =>
    set((state) => ({ bottomPanelOpen: !state.bottomPanelOpen })),

  setCommandPaletteOpen: (open) => set({ commandPaletteOpen: open }),
}))

if (typeof window !== 'undefined') {
  applyTheme(useUIStore.getState().theme)
}
