import { useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { create } from 'zustand'
import type { ApiResponse, Assertion, TestResult } from '../types'

interface TestState {
  assertions: Assertion[]
  results: TestResult[] | null
  loading: boolean

  setAssertions: (assertions: Assertion[]) => void
  addAssertion: (assertion: Assertion) => void
  removeAssertion: (id: string) => void
  updateAssertion: (id: string, patch: Partial<Assertion>) => void
  setResults: (results: TestResult[] | null) => void
  setLoading: (loading: boolean) => void
  reset: () => void
}

const useTestStore = create<TestState>((set) => ({
  assertions: [],
  results: null,
  loading: false,

  setAssertions: (assertions) => set({ assertions }),
  addAssertion: (assertion) =>
    set((state) => ({ assertions: [...state.assertions, assertion] })),
  removeAssertion: (id) =>
    set((state) => ({
      assertions: state.assertions.filter((a) => a.id !== id),
    })),
  updateAssertion: (id, patch) =>
    set((state) => ({
      assertions: state.assertions.map((a) =>
        a.id === id ? { ...a, ...patch } : a
      ),
    })),
  setResults: (results) => set({ results }),
  setLoading: (loading) => set({ loading }),
  reset: () => set({ assertions: [], results: null, loading: false }),
}))

/**
 * Hook for building assertions and running them against an API response.
 *
 * State (assertions, results) lives in a Zustand store so any component
 * can subscribe to test changes.
 */
export function useTestRunner() {
  const {
    assertions,
    results,
    loading,
    addAssertion,
    removeAssertion,
    updateAssertion,
    setResults,
    setLoading,
    reset,
  } = useTestStore()

  const runTests = useCallback(
    async (response: ApiResponse) => {
      if (assertions.length === 0) {
        setResults([])
        return
      }

      setLoading(true)
      try {
        const result = await invoke<TestResult>('run_tests', {
          response,
          suiteName: 'Request Tests',
          assertions: assertions.map((a) => ({
            name: a.name,
            ...a.config,
          })),
        })
        setResults([result])
      } catch (err) {
        console.error('Test run failed:', err)
        setResults(null)
      } finally {
        setLoading(false)
      }
    },
    [assertions, setLoading, setResults]
  )

  const addQuickAssertion = useCallback(
    (kind: Assertion['config']['type']) => {
      const baseConfig = (() => {
        switch (kind) {
          case 'status_code':
            return { type: 'status_code' as const, expected: 200 }
          case 'response_time':
            return { type: 'response_time' as const, maxMs: 1000 }
          case 'body_contains':
            return { type: 'body_contains' as const, substring: '' }
          case 'body_matches':
            return { type: 'body_matches' as const, pattern: '' }
          case 'header_equals':
            return { type: 'header_equals' as const, header: '', expected: '' }
          case 'header_contains':
            return { type: 'header_contains' as const, header: '', substring: '' }
          case 'json_path':
            return { type: 'json_path' as const, path: '$.data.id', expected: '' }
          case 'content_type':
            return { type: 'content_type' as const, expected: 'application/json' }
        }
      })()

      addAssertion({
        id: crypto.randomUUID(),
        name: defaultName(kind),
        config: baseConfig,
      })
    },
    [addAssertion]
  )

  return {
    assertions,
    results,
    loading,
    addQuickAssertion,
    removeAssertion,
    updateAssertion,
    runTests,
    reset,
  }
}

function defaultName(kind: Assertion['config']['type']): string {
  switch (kind) {
    case 'status_code':
      return 'Status is 200'
    case 'response_time':
      return 'Response time < 1s'
    case 'body_contains':
      return 'Body contains…'
    case 'body_matches':
      return 'Body matches regex…'
    case 'header_equals':
      return 'Header equals…'
    case 'header_contains':
      return 'Header contains…'
    case 'json_path':
      return 'JSON path…'
    case 'content_type':
      return 'Content-Type is JSON'
  }
}
