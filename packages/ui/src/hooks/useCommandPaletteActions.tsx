import { useMemo } from 'react'
import {
  Send,
  Save,
  FolderOpen,
  History as HistoryIcon,
  Settings as SettingsIcon,
  Sun,
  Moon,
  Monitor,
  Plus,
  Trash2,
  Copy,
  Code2,
  Terminal,
  Globe,
} from 'lucide-react'
import { useUIStore } from '../stores/uiStore'
import { useEnvironmentStore } from '../stores/environmentStore'
import { useHistoryStore } from '../stores/historyStore'
import type { Command } from '../components/dialogs/CommandPalette'
import type { Request } from '../types'

export interface UseCommandPaletteActionsParams {
  currentRequest: Request | null
  onSend?: () => void | Promise<void>
  onSaveRequest?: () => void | Promise<void>
  onOpenImport?: () => void
  onOpenEnvManager?: () => void
  onOpenHistory?: () => void
  onOpenSettings?: () => void
  onOpenCodeSnippet?: () => void
  onCreateNewRequest?: () => void
  onClearHistory?: () => void
}

/**
 * Returns the list of commands available in the command palette, wired to
 * the relevant stores and the per-app callbacks.
 */
export function useCommandPaletteActions(
  params: UseCommandPaletteActionsParams
): Command[] {
  const setTheme = useUIStore((s) => s.setTheme)
  const toggleSidebar = useUIStore((s) => s.toggleSidebar)
  const toggleBottomPanel = useUIStore((s) => s.toggleBottomPanel)
  const environments = useEnvironmentStore((s) => s.environments)
  const setActiveEnvironment = useEnvironmentStore((s) => s.setActiveEnvironment)
  const activeEnvironmentId = useEnvironmentStore((s) => s.activeEnvironmentId)
  const history = useHistoryStore((s) => s.items)
  const replayHistoryItem = useHistoryStore((s) => s.replay)

  return useMemo<Command[]>(() => {
    const commands: Command[] = []

    // Request actions
    if (params.onSend) {
      commands.push({
        id: 'request.send',
        label: 'Send Request',
        description: 'Execute the current request',
        group: 'Request',
        shortcut: '⌘↵',
        icon: <Send className="h-3 w-3" />,
        keywords: ['run', 'execute', 'go'],
        perform: () => params.onSend?.(),
      })
    }
    if (params.onSaveRequest) {
      commands.push({
        id: 'request.save',
        label: 'Save Request',
        description: 'Persist changes to the active collection',
        group: 'Request',
        shortcut: '⌘S',
        icon: <Save className="h-3 w-3" />,
        keywords: ['persist', 'write'],
        perform: () => params.onSaveRequest?.(),
      })
    }
    if (params.onCreateNewRequest) {
      commands.push({
        id: 'request.new',
        label: 'New Request',
        description: 'Open a blank request',
        group: 'Request',
        icon: <Plus className="h-3 w-3" />,
        keywords: ['create', 'add'],
        perform: () => params.onCreateNewRequest?.(),
      })
    }
    if (params.onOpenCodeSnippet) {
      commands.push({
        id: 'request.code',
        label: 'Generate Code Snippet',
        description: 'View this request as curl, fetch, axios…',
        group: 'Request',
        icon: <Code2 className="h-3 w-3" />,
        keywords: ['snippet', 'curl', 'export'],
        perform: () => params.onOpenCodeSnippet?.(),
      })
    }

    // Environments
    for (const env of environments) {
      commands.push({
        id: `env.use.${env.id}`,
        label: `Use environment: ${env.name}`,
        description:
          env.id === activeEnvironmentId
            ? '(currently active)'
            : 'Switch active environment',
        group: 'Environments',
        icon: <Globe className="h-3 w-3" />,
        keywords: ['switch', 'active', 'variables'],
        perform: () => setActiveEnvironment(env.id),
      })
    }
    if (params.onOpenEnvManager) {
      commands.push({
        id: 'env.manager',
        label: 'Manage Environments…',
        group: 'Environments',
        icon: <FolderOpen className="h-3 w-3" />,
        keywords: ['create', 'edit', 'delete'],
        perform: () => params.onOpenEnvManager?.(),
      })
    }

    // History
    if (params.onOpenHistory) {
      commands.push({
        id: 'history.open',
        label: 'Open History',
        description: 'Browse past requests',
        group: 'History',
        shortcut: '⌘H',
        icon: <HistoryIcon className="h-3 w-3" />,
        keywords: ['log', 'past', 'recent'],
        perform: () => params.onOpenHistory?.(),
      })
    }
    if (params.onClearHistory) {
      commands.push({
        id: 'history.clear',
        label: 'Clear All History',
        group: 'History',
        icon: <Trash2 className="h-3 w-3" />,
        keywords: ['delete', 'reset'],
        perform: () => params.onClearHistory?.(),
      })
    }
    for (const item of history.slice(0, 8)) {
      commands.push({
        id: `history.replay.${item.id}`,
        label: `Replay: ${item.method} ${truncate(item.url, 40)}`,
        description: new Date(item.timestamp).toLocaleString(),
        group: 'History',
        icon: <Copy className="h-3 w-3" />,
        keywords: ['repeat', 'replay'],
        perform: () => replayHistoryItem(item.id),
      })
    }

    // Theme
    commands.push({
      id: 'theme.light',
      label: 'Theme: Light',
      group: 'Appearance',
      icon: <Sun className="h-3 w-3" />,
      perform: () => setTheme('light'),
    })
    commands.push({
      id: 'theme.dark',
      label: 'Theme: Dark',
      group: 'Appearance',
      icon: <Moon className="h-3 w-3" />,
      perform: () => setTheme('dark'),
    })
    commands.push({
      id: 'theme.system',
      label: 'Theme: System',
      group: 'Appearance',
      icon: <Monitor className="h-3 w-3" />,
      perform: () => setTheme('system'),
    })

    // Layout
    commands.push({
      id: 'layout.sidebar',
      label: 'Toggle Sidebar',
      group: 'Layout',
      icon: <Terminal className="h-3 w-3" />,
      keywords: ['panel'],
      perform: () => toggleSidebar(),
    })
    commands.push({
      id: 'layout.bottom',
      label: 'Toggle Bottom Panel',
      group: 'Layout',
      icon: <Terminal className="h-3 w-3" />,
      keywords: ['console', 'tests'],
      perform: () => toggleBottomPanel(),
    })

    // Misc
    if (params.onOpenImport) {
      commands.push({
        id: 'import.open',
        label: 'Import Collection…',
        description: 'Postman, cURL, Insomnia, or Bruno',
        group: 'File',
        icon: <FolderOpen className="h-3 w-3" />,
        keywords: ['open', 'load'],
        perform: () => params.onOpenImport?.(),
      })
    }
    if (params.onOpenSettings) {
      commands.push({
        id: 'settings.open',
        label: 'Open Settings',
        group: 'File',
        shortcut: '⌘,',
        icon: <SettingsIcon className="h-3 w-3" />,
        keywords: ['preferences'],
        perform: () => params.onOpenSettings?.(),
      })
    }

    return commands
  }, [
    environments,
    activeEnvironmentId,
    history,
    setTheme,
    toggleSidebar,
    toggleBottomPanel,
    setActiveEnvironment,
    replayHistoryItem,
    params,
  ])
}

function truncate(s: string, n: number) {
  return s.length > n ? s.slice(0, n - 1) + '…' : s
}
