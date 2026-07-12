import * as React from 'react'
import { cn } from '../../lib/utils'
import { ActivityBar, type ActivityBarItem } from './ActivityBar'
import { Sidebar } from './Sidebar'
import { TabBar, type Tab } from './TabBar'
import { PanelLayout } from './PanelLayout'
import { StatusBar, type StatusBarItem } from './StatusBar'

export interface AppShellProps {
  /** Activity bar items and active state */
  activityItems?: ActivityBarItem[]
  activeActivity?: string | null
  onActivitySelect?: (id: string) => void

  /** Sidebar content (shown between activity bar and editor) */
  sidebar?: React.ReactNode
  sidebarWidth?: number

  /** Tab bar */
  tabs?: Tab[]
  activeTab?: string | null
  onTabSelect?: (id: string) => void
  onTabClose?: (id: string) => void
  onTabReorder?: (fromIndex: number, toIndex: number) => void

  /** Main editor content */
  children: React.ReactNode

  /** Bottom panel content */
  bottomPanel?: React.ReactNode

  /** Status bar */
  statusBarItems?: StatusBarItem[]
  statusLeft?: string
  statusRight?: string

  className?: string
}

export const AppShell = React.forwardRef<HTMLDivElement, AppShellProps>(
  (
    {
      activityItems,
      activeActivity,
      onActivitySelect,
      sidebar,
      sidebarWidth,
      tabs,
      activeTab,
      onTabSelect,
      onTabClose,
      onTabReorder,
      children,
      bottomPanel,
      statusBarItems,
      statusLeft,
      statusRight,
      className,
    },
    ref
  ) => {
    return (
      <div
        ref={ref}
        className={cn('flex h-screen w-screen overflow-hidden bg-white dark:bg-gray-950 text-gray-900 dark:text-gray-100', className)}
        data-testid="app-shell"
      >
        {/* Activity bar */}
        <ActivityBar
          items={activityItems}
          activeId={activeActivity}
          onSelect={onActivitySelect}
        />

        {/* Sidebar */}
        {sidebar && (
          <Sidebar width={sidebarWidth}>
            {sidebar}
          </Sidebar>
        )}

        {/* Main content area */}
        <div className="flex flex-col flex-1 min-w-0">
          {tabs && tabs.length > 0 && (
            <TabBar
              tabs={tabs}
              activeId={activeTab ?? null}
              onSelect={(id) => onTabSelect?.(id)}
              onClose={(id) => onTabClose?.(id)}
              onReorder={onTabReorder}
            />
          )}

          <PanelLayout bottom={bottomPanel}>
            {children}
          </PanelLayout>

          <StatusBar
            items={statusBarItems}
            leftText={statusLeft}
            rightText={statusRight}
          />
        </div>
      </div>
    )
  }
)

AppShell.displayName = 'AppShell'
