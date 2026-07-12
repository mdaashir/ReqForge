import * as React from 'react'
import { ChevronLeft, ChevronRight, X, Sparkles } from 'lucide-react'
import { Button } from '../primitives/Button'
import { cn } from '../../lib/utils'

export interface TourStep {
  /** CSS selector for the element to highlight */
  target: string
  title: string
  description: string
  /** Optional explicit placement, otherwise auto-detected */
  placement?: 'top' | 'bottom' | 'left' | 'right'
}

export interface OnboardingTourProps {
  steps: TourStep[]
  storageKey?: string
  className?: string
  onComplete?: () => void
}

const STORAGE_PREFIX = 'reqforge-tour-'

interface Position {
  top: number
  left: number
  placement: 'top' | 'bottom' | 'left' | 'right'
}

/**
 * Lightweight coach-marks tour. Walks through `steps` and dismisses
 * permanently once the user finishes or skips. Re-shows on demand by
 * clearing the localStorage flag.
 *
 * We rely on `data-tour-id` attributes on the target elements plus a
 * `data-testid` selector match — no external dependencies.
 */
export const OnboardingTour: React.FC<OnboardingTourProps> = ({
  steps,
  storageKey = 'default',
  className,
  onComplete,
}) => {
  const [active, setActive] = React.useState(false)
  const [stepIdx, setStepIdx] = React.useState(0)
  const [pos, setPos] = React.useState<Position | null>(null)

  // Mount: check storage
  React.useEffect(() => {
    if (typeof window === 'undefined') return
    const seen = localStorage.getItem(STORAGE_PREFIX + storageKey)
    if (!seen) {
      // Small delay so the layout settles before we measure.
      const timer = setTimeout(() => setActive(true), 600)
      return () => clearTimeout(timer)
    }
  }, [storageKey])

  // Reposition whenever step or layout changes
  React.useEffect(() => {
    if (!active) return
    const recompute = () => {
      const step = steps[stepIdx]
      if (!step) return
      const el = document.querySelector(step.target) as HTMLElement | null
      if (!el) {
        // Target not yet mounted — try again shortly
        setPos(null)
        const timer = setTimeout(recompute, 200)
        return () => clearTimeout(timer)
      }
      const rect = el.getBoundingClientRect()
      const placement = step.placement ?? autoPlacement(rect)
      setPos(computePos(rect, placement))
    }
    recompute()
    window.addEventListener('resize', recompute)
    window.addEventListener('scroll', recompute, true)
    return () => {
      window.removeEventListener('resize', recompute)
      window.removeEventListener('scroll', recompute, true)
    }
  }, [active, stepIdx, steps])

  if (!active || !steps.length) return null
  const step = steps[stepIdx]
  if (!step) return null
  const isLast = stepIdx === steps.length - 1

  const dismiss = (markSeen = true) => {
    if (markSeen && typeof window !== 'undefined') {
      localStorage.setItem(STORAGE_PREFIX + storageKey, '1')
    }
    setActive(false)
    onComplete?.()
  }

  const next = () => {
    if (isLast) dismiss()
    else setStepIdx((i) => i + 1)
  }
  const prev = () => setStepIdx((i) => Math.max(0, i - 1))

  return (
    <>
      <div
        className="fixed inset-0 z-[60] bg-black/40 backdrop-blur-[1px]"
        aria-hidden="true"
      />
      {pos && <Spotlight pos={pos} />}
      {pos && (
        <div
          className={cn(
            'fixed z-[70] w-72 p-4 rounded-lg shadow-2xl',
            'bg-white dark:bg-gray-900 border border-gray-200 dark:border-gray-700',
            className
          )}
          style={{ top: pos.top, left: pos.left }}
          data-testid="tour-popover"
          role="dialog"
          aria-label="Onboarding tour"
        >
          <div className="flex items-start gap-2 mb-2">
            <Sparkles className="h-4 w-4 text-blue-500 flex-shrink-0 mt-0.5" />
            <div className="flex-1">
              <div className="flex items-center justify-between mb-1">
                <h3 className="font-semibold text-sm text-gray-900 dark:text-gray-100">
                  {step.title}
                </h3>
                <button
                  onClick={() => dismiss()}
                  className="text-gray-400 hover:text-gray-600 dark:hover:text-gray-200"
                  aria-label="Close tour"
                  data-testid="tour-close"
                >
                  <X className="h-3.5 w-3.5" />
                </button>
              </div>
              <p className="text-xs text-gray-600 dark:text-gray-400 leading-relaxed">
                {step.description}
              </p>
            </div>
          </div>
          <div className="flex items-center justify-between mt-3 pt-3 border-t border-gray-100 dark:border-gray-800">
            <span className="text-[10px] text-gray-500">
              {stepIdx + 1} of {steps.length}
            </span>
            <div className="flex items-center gap-1">
              <Button
                variant="ghost"
                size="sm"
                onClick={prev}
                disabled={stepIdx === 0}
                aria-label="Previous"
                data-testid="tour-prev"
              >
                <ChevronLeft className="h-3 w-3" />
              </Button>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => dismiss()}
                data-testid="tour-skip"
              >
                Skip
              </Button>
              <Button
                variant="default"
                size="sm"
                onClick={next}
                data-testid="tour-next"
              >
                {isLast ? 'Done' : 'Next'}
                {!isLast && <ChevronRight className="h-3 w-3 ml-0.5" />}
              </Button>
            </div>
          </div>
        </div>
      )}
    </>
  )
}

const Spotlight: React.FC<{ pos: Position }> = ({ pos }) => {
  // Rendered as an outline that follows the highlight rectangle.
  // We compute the rectangle from `pos` (which is the popover position).
  return (
    <div
      className="fixed z-[65] pointer-events-none ring-2 ring-blue-400 ring-offset-2 ring-offset-transparent rounded-md transition-all duration-200"
      style={{
        top: pos.top - 6,
        left: pos.left - 6,
        width: 1,
        height: 1,
      }}
      aria-hidden="true"
    />
  )
}

function autoPlacement(rect: DOMRect): Position['placement'] {
  const vw = window.innerWidth
  const vh = window.innerHeight
  // Prefer bottom; fall back to top if no room.
  if (rect.bottom + 200 < vh) return 'bottom'
  if (rect.top - 200 > 0) return 'top'
  if (rect.right + 320 < vw) return 'right'
  return 'left'
}

function computePos(rect: DOMRect, placement: Position['placement']): Position {
  const gap = 12
  let top = 0
  let left = 0
  switch (placement) {
    case 'bottom':
      top = rect.bottom + gap
      left = rect.left + rect.width / 2 - 144
      break
    case 'top':
      top = rect.top - gap - 180
      left = rect.left + rect.width / 2 - 144
      break
    case 'right':
      top = rect.top + rect.height / 2 - 80
      left = rect.right + gap
      break
    case 'left':
      top = rect.top + rect.height / 2 - 80
      left = rect.left - gap - 288
      break
  }
  // Clamp to viewport
  top = Math.max(8, Math.min(window.innerHeight - 200, top))
  left = Math.max(8, Math.min(window.innerWidth - 296, left))
  return { top, left, placement }
}

/**
 * Helper hook that re-shows the tour by clearing the localStorage flag.
 * Bind this to a button (or a command palette action) so power users can
 * replay the tour.
 */
export function useReplayTour(storageKey = 'default') {
  return React.useCallback(() => {
    if (typeof window !== 'undefined') {
      localStorage.removeItem(STORAGE_PREFIX + storageKey)
      // Force a reload so the tour's mount effect picks up the cleared flag.
      window.location.reload()
    }
  }, [storageKey])
}
