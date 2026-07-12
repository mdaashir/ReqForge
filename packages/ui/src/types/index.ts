/**
 * HTTP method types
 */
export type HttpMethod =
  | 'GET'
  | 'POST'
  | 'PUT'
  | 'PATCH'
  | 'DELETE'
  | 'HEAD'
  | 'OPTIONS'
  | 'TRACE'
  | 'CONNECT'
  | (string & {})

/**
 * Key-value pair (used for headers, query params, form fields)
 */
export interface KeyValue {
  key: string
  value: string
  enabled: boolean
  description?: string
}

/**
 * Request body types
 */
export type BodyMode = 'none' | 'json' | 'xml' | 'text' | 'form' | 'multipart' | 'binary' | 'graphql'

export interface RequestBody {
  contentType?: string
  content: string
  mode: BodyMode
}

/**
 * Authentication types
 */
export type AuthType =
  | 'none'
  | 'apiKey'
  | 'bearer'
  | 'basic'
  | 'oauth2'
  | 'jwt'
  | 'awsSigV4'
  | (string & {})

export interface AuthConfig {
  type: AuthType
  config: Record<string, string>
}

/**
 * Request structure
 */
export interface Request {
  id: string
  name: string
  method: HttpMethod
  url: string
  headers: KeyValue[]
  params: KeyValue[]
  body: RequestBody
  auth?: AuthConfig
  timeout?: number
  followRedirects: boolean
  verifySSL: boolean
  preRequestScript?: string
  postResponseScript?: string
  testScript?: string
  description?: string
}

/**
 * Response timing breakdown
 */
export interface ResponseTiming {
  dnsMs: number
  connectMs: number
  tlsMs: number
  sendMs: number
  waitMs: number
  receiveMs: number
  totalMs: number
}

/**
 * Response size info
 */
export interface ResponseSize {
  headers: number
  body: number
  total: number
}

/**
 * Cookie
 */
export interface Cookie {
  name: string
  value: string
  domain: string
  path: string
  secure: boolean
  httpOnly: boolean
  expires?: number
}

/**
 * Response structure
 */
export interface ApiResponse {
  status: number
  statusText: string
  headers: Record<string, string>
  body: string
  cookies: Cookie[]
  timing: ResponseTiming
  size: ResponseSize
  url: string
  protocol: string
  contentType?: string
  isText: boolean
}

/**
 * Collection item (folder or request)
 */
export interface CollectionItem {
  id: string
  name: string
  type: 'folder' | 'request'
  children?: CollectionItem[]
  request?: Request
}

/**
 * Collection
 */
export interface Collection {
  id: string
  name: string
  description?: string
  items: CollectionItem[]
  auth?: AuthConfig
  headers?: KeyValue[]
  variables?: KeyValue[]
  createdAt: number
  updatedAt: number
}

/**
 * Variable in an environment
 */
export interface Variable {
  key: string
  value: string
  var_type: 'string' | 'number' | 'boolean' | 'secret'
  enabled: boolean
}

/**
 * Environment
 */
export interface Environment {
  id: string
  name: string
  variables: Variable[]
  color?: string
  created_at: string
  updated_at: string
}

/**
 * Test assertion types (mirrors Rust enum)
 */
export type AssertionConfig =
  | { type: 'status_code'; expected: number }
  | { type: 'response_time'; maxMs: number }
  | { type: 'body_contains'; substring: string }
  | { type: 'body_matches'; pattern: string }
  | { type: 'header_equals'; header: string; expected: string }
  | { type: 'header_contains'; header: string; substring: string }
  | { type: 'json_path'; path: string; expected: string }
  | { type: 'content_type'; expected: string }

export interface Assertion {
  id: string
  name: string
  config: AssertionConfig
}

export type TestStatus = 'passed' | 'failed' | 'skipped' | 'error'

export interface TestAssertionResult {
  passed: boolean
  message: string
  expected?: string
  actual?: string
}

export interface TestResult {
  name: string
  status: TestStatus
  assertions: TestAssertionResult[]
  durationMs: number
}

/**
 * Theme
 */
export type Theme = 'light' | 'dark' | 'system'
