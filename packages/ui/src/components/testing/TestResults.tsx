import * as React from 'react'
import { CheckCircle2, XCircle, Clock } from 'lucide-react'
import { cn } from '../../lib/utils'
import type { TestResult } from '../../types'

export interface TestResultsProps {
  results: TestResult[] | null
  loading?: boolean
  className?: string
}

const StatusBadge = ({ status }: { status: TestResult['status'] }) => {
  const styles: Record<TestResult['status'], string> = {
    passed: 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400',
    failed: 'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400',
    skipped: 'bg-gray-100 text-gray-700 dark:bg-gray-800 dark:text-gray-300',
    error: 'bg-orange-100 text-orange-700 dark:bg-orange-900/30 dark:text-orange-400',
  }
  return (
    <span
      className={cn(
        'inline-flex items-center px-2 py-0.5 rounded text-xs font-semibold uppercase',
        styles[status]
      )}
    >
      {status}
    </span>
  )
}

export const TestResults = React.forwardRef<HTMLDivElement, TestResultsProps>(
  ({ results, loading, className }, ref) => {
    if (loading) {
      return (
        <div ref={ref} className={cn('p-4 text-sm text-gray-500', className)}>
          Running tests...
        </div>
      )
    }

    if (!results || results.length === 0) {
      return (
        <div
          ref={ref}
          className={cn('p-4 text-sm text-gray-500 dark:text-gray-400', className)}
          data-testid="test-results-empty"
        >
          No test results yet. Add assertions to your request.
        </div>
      )
    }

    const passed = results.filter((r) => r.status === 'passed').length
    const failed = results.filter((r) => r.status === 'failed').length
    const total = results.length

    return (
      <div
        ref={ref}
        className={cn('flex flex-col gap-3', className)}
        data-testid="test-results"
      >
        <div className="flex items-center gap-4 text-sm">
          <div className="flex items-center gap-1.5">
            <CheckCircle2 className="h-4 w-4 text-green-600" />
            <span className="font-semibold">{passed}</span>
            <span className="text-gray-500">passed</span>
          </div>
          <div className="flex items-center gap-1.5">
            <XCircle className="h-4 w-4 text-red-600" />
            <span className="font-semibold">{failed}</span>
            <span className="text-gray-500">failed</span>
          </div>
          <div className="text-gray-500">
            {total} {total === 1 ? 'test' : 'tests'}
          </div>
        </div>

        <div className="flex flex-col gap-2">
          {results.map((result, idx) => (
            <div
              key={idx}
              className="p-3 rounded-md border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800"
              data-testid={`test-result-${idx}`}
            >
              <div className="flex items-center justify-between mb-2">
                <div className="flex items-center gap-2">
                  {result.status === 'passed' ? (
                    <CheckCircle2 className="h-4 w-4 text-green-600" />
                  ) : (
                    <XCircle className="h-4 w-4 text-red-600" />
                  )}
                  <span className="font-semibold text-sm">{result.name}</span>
                  <StatusBadge status={result.status} />
                </div>
                <div className="flex items-center gap-1 text-xs text-gray-500">
                  <Clock className="h-3 w-3" />
                  {result.durationMs}ms
                </div>
              </div>

              <ul className="flex flex-col gap-1 ml-6 text-xs">
                {result.assertions.map((a, aIdx) => (
                  <li
                    key={aIdx}
                    className={cn(
                      'flex items-start gap-1.5',
                      a.passed ? 'text-gray-700 dark:text-gray-300' : 'text-red-600 dark:text-red-400'
                    )}
                  >
                    {a.passed ? (
                      <CheckCircle2 className="h-3 w-3 mt-0.5 flex-shrink-0 text-green-600" />
                    ) : (
                      <XCircle className="h-3 w-3 mt-0.5 flex-shrink-0 text-red-600" />
                    )}
                    <span className="flex-1">{a.message}</span>
                  </li>
                ))}
              </ul>
            </div>
          ))}
        </div>
      </div>
    )
  }
)

TestResults.displayName = 'TestResults'
