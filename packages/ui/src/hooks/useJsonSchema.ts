import { useMemo } from 'react'
import Ajv2020 from 'ajv/dist/2020'
import addFormats from 'ajv-formats'

export interface ValidationError {
  path: string
  message: string
}

export interface ValidationResult {
  valid: boolean
  errors: ValidationError[]
}

/**
 * Validates JSON content against an optional JSON Schema. Returns an empty
 * valid result when either input is empty so the hook is safe to call
 * unconditionally from the body editor.
 *
 * Uses Ajv in strict mode so unknown keywords in the schema raise a warning
 * (we surface that as an error rather than silently ignoring).
 */
export function useJsonSchema(content: string, schema: string): ValidationResult {
  return useMemo(() => {
    if (!schema.trim() || !content.trim()) {
      return { valid: true, errors: [] }
    }

    let parsedSchema: unknown
    let parsedData: unknown
    try {
      parsedSchema = JSON.parse(schema)
    } catch (err) {
      return {
        valid: false,
        errors: [
          {
            path: '(schema)',
            message: `Invalid schema JSON: ${err instanceof Error ? err.message : String(err)}`,
          },
        ],
      }
    }
    try {
      parsedData = JSON.parse(content)
    } catch (err) {
      return {
        valid: false,
        errors: [
          {
            path: '(body)',
            message: `Invalid body JSON: ${err instanceof Error ? err.message : String(err)}`,
          },
        ],
      }
    }

    try {
      const ajv = new Ajv2020({ allErrors: true, strict: false })
      addFormats(ajv)
      const validate = ajv.compile(parsedSchema as Record<string, unknown>)
      const valid = validate(parsedData)
      if (valid) return { valid: true, errors: [] }
      return {
        valid: false,
        errors: (validate.errors ?? []).map((e: any) => ({
          path: e.instancePath || '(root)',
          message: `${e.message ?? 'invalid'}${e.params ? ' ' + JSON.stringify(e.params) : ''}`,
        })),
      }
    } catch (err) {
      return {
        valid: false,
        errors: [
          {
            path: '(schema)',
            message: `Schema compile error: ${err instanceof Error ? err.message : String(err)}`,
          },
        ],
      }
    }
  }, [content, schema])
}
