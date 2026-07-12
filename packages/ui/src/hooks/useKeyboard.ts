import { useEffect } from 'react'

export interface Keybinding {
  /** Key like 'k', 'Enter', 'ArrowDown' */
  key: string
  /** Require ctrl on Windows/Linux */
  ctrl?: boolean
  /** Require meta (cmd) on macOS */
  meta?: boolean
  /** Require shift */
  shift?: boolean
  /** Require alt */
  alt?: boolean
  handler: (e: KeyboardEvent) => void
  /** Prevent default behaviour; default true for letter keys */
  preventDefault?: boolean
}

/**
 * Global keyboard shortcut hook.
 *
 * Bind shortcuts at the window level. Each Keybinding is checked against
 * KeyboardEvent properties; match requires the same modifiers and key.
 */
export function useKeyboard(bindings: Keybinding[], enabled: boolean = true) {
  useEffect(() => {
    if (!enabled) return

    const handler = (e: KeyboardEvent) => {
      for (const b of bindings) {
        if (e.key.toLowerCase() !== b.key.toLowerCase()) continue
        if (!!b.ctrl !== e.ctrlKey) continue
        if (!!b.meta !== e.metaKey) continue
        if (!!b.shift !== e.shiftKey) continue
        if (!!b.alt !== e.altKey) continue

        if (b.preventDefault ?? true) {
          e.preventDefault()
        }
        b.handler(e)
        break
      }
    }

    window.addEventListener('keydown', handler)
    return () => window.removeEventListener('keydown', handler)
  }, [bindings, enabled])
}
