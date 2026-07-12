import { create } from 'zustand'
import { persist } from 'zustand/middleware'

export type ResponseBodyView = 'pretty' | 'raw' | 'preview'
export type ProxyType = 'none' | 'http' | 'socks5'

export interface ProxyConfig {
  type: ProxyType
  url: string
  bypass?: string
}

export interface Settings {
  // Appearance
  theme: 'light' | 'dark' | 'system'
  fontFamily: string
  fontSize: number

  // Editor
  tabSize: number
  wordWrap: boolean
  minimap: boolean

  // Request behaviour
  defaultTimeoutMs: number
  followRedirects: boolean
  verifySsl: boolean
  sendCookies: boolean

  // Response viewer
  responseBodyView: ResponseBodyView
  showResponseTiming: boolean

  // Network
  proxy: ProxyConfig
  userAgent: string

  // Privacy
  telemetry: boolean
  crashReports: boolean
}

const defaults: Settings = {
  theme: 'system',
  fontFamily: 'Inter, system-ui, sans-serif',
  fontSize: 14,

  tabSize: 2,
  wordWrap: true,
  minimap: false,

  defaultTimeoutMs: 30000,
  followRedirects: true,
  verifySsl: true,
  sendCookies: true,

  responseBodyView: 'pretty',
  showResponseTiming: true,

  proxy: { type: 'none', url: '' },
  userAgent: 'ReqForge/0.1.0',

  telemetry: false,
  crashReports: false,
}

interface SettingsState extends Settings {
  update: <K extends keyof Settings>(key: K, value: Settings[K]) => void
  updateMany: (patch: Partial<Settings>) => void
  reset: () => void
}

export const useSettingsStore = create<SettingsState>()(
  persist(
    (set) => ({
      ...defaults,
      update: (key, value) => set({ [key]: value } as Partial<Settings>),
      updateMany: (patch) => set(patch),
      reset: () => set(defaults),
    }),
    {
      name: 'reqforge-settings',
      version: 1,
    }
  )
)
