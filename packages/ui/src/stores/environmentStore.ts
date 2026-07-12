import { create } from 'zustand'
import type { Environment } from '../types'

interface EnvironmentState {
  environments: Environment[]
  activeEnvironmentId: string | null

  setEnvironments: (environments: Environment[]) => void
  addEnvironment: (environment: Environment) => void
  updateEnvironment: (id: string, updates: Partial<Environment>) => void
  removeEnvironment: (id: string) => void
  setActiveEnvironment: (id: string | null) => void
  getActiveEnvironment: () => Environment | undefined
  getVariable: (key: string) => string | undefined
  setVariable: (envId: string, key: string, value: string) => void
}

export const useEnvironmentStore = create<EnvironmentState>((set, get) => ({
  environments: [],
  activeEnvironmentId: null,

  setEnvironments: (environments) => set({ environments }),

  addEnvironment: (environment) =>
    set((state) => ({ environments: [...state.environments, environment] })),

  updateEnvironment: (id, updates) =>
    set((state) => ({
      environments: state.environments.map((env) =>
        env.id === id ? { ...env, ...updates } : env
      ),
    })),

  removeEnvironment: (id) =>
    set((state) => ({
      environments: state.environments.filter((env) => env.id !== id),
      activeEnvironmentId:
        state.activeEnvironmentId === id ? null : state.activeEnvironmentId,
    })),

  setActiveEnvironment: (id) => set({ activeEnvironmentId: id }),

  getActiveEnvironment: () => {
    const { environments, activeEnvironmentId } = get()
    return environments.find((env) => env.id === activeEnvironmentId)
  },

  getVariable: (key) => {
    const env = get().getActiveEnvironment()
    if (!env) return undefined
    const variable = env.variables.find((v) => v.enabled && v.key === key)
    return variable?.value
  },

  setVariable: (envId, key, value) =>
    set((state) => ({
      environments: state.environments.map((env) =>
        env.id === envId
          ? {
              ...env,
              variables: env.variables.map((v) =>
                v.key === key ? { ...v, value } : v
              ),
            }
          : env
      ),
    })),
}))
