import type { KeyValue, Request, AuthConfig } from '../types'

export type SnippetLanguage =
  | 'curl'
  | 'fetch'
  | 'axios'
  | 'python'
  | 'go'
  | 'ruby'
  | 'powershell'
  | 'java'

export interface SnippetOptions {
  indentSize?: number
  includeComments?: boolean
}

/**
 * Convert a request into runnable code for a given language.
 *
 * Supported languages: cURL, fetch (browser), axios (Node), Python requests,
 * Go net/http, Ruby Net::HTTP, PowerShell Invoke-WebRequest, Java HttpClient.
 */
export function generateSnippet(
  request: Request,
  language: SnippetLanguage,
  options: SnippetOptions = {}
): string {
  const { includeComments = true } = options
  const generator = generators[language]
  if (!generator) {
    throw new Error(`Unsupported language: ${language}`)
  }
  return generator(request, includeComments)
}

// === Helpers ===

function shellQuote(s: string): string {
  // Wrap in single quotes, escape any embedded single quotes
  return `'${s.replace(/'/g, `'\\''`)}'`
}

function needsBody(method: string): boolean {
  return !['GET', 'HEAD'].includes(method.toUpperCase())
}

function getBodyString(body: Request['body']): string {
  return body?.content ?? ''
}

function applyAuthToHeaders(
  headers: KeyValue[],
  auth?: AuthConfig
): KeyValue[] {
  if (!auth || auth.type === 'none') return headers

  // Remove any existing Authorization header to avoid duplicates
  const filtered = headers.filter(
    (h) => h.key.toLowerCase() !== 'authorization'
  )

  switch (auth.type) {
    case 'bearer':
      if (auth.config.token) {
        filtered.push({
          key: 'Authorization',
          value: `Bearer ${auth.config.token}`,
          enabled: true,
        })
      }
      break
    case 'basic': {
      const creds = `${auth.config.username ?? ''}:${auth.config.password ?? ''}`
      const encoded = btoa(creds)
      filtered.push({
        key: 'Authorization',
        value: `Basic ${encoded}`,
        enabled: true,
      })
      break
    }
    case 'apiKey':
      if (auth.config.location === 'header' && auth.config.key) {
        filtered.push({
          key: auth.config.key,
          value: auth.config.value ?? '',
          enabled: true,
        })
      }
      break
  }
  return filtered
}

// === Generators ===

const generators: Record<SnippetLanguage, (req: Request, comments: boolean) => string> = {
  curl: (req, comments) => {
    const lines: string[] = []
    if (comments) lines.push(`# ${req.method.toUpperCase()} ${req.url}`)

    const headers = applyAuthToHeaders(req.headers, req.auth)
    const url = req.url
    const body = getBodyString(req.body)
    const hasBody = needsBody(req.method) && body.length > 0

    if (hasBody) lines.push(`curl -X ${req.method.toUpperCase()} ${shellQuote(url)}`)
    else lines.push(`curl ${shellQuote(url)}`)

    for (const h of headers.filter((h) => h.enabled)) {
      lines.push(`  -H ${shellQuote(`${h.key}: ${h.value}`)}`)
    }

    if (req.params.length > 0) {
      const qs = req.params
        .filter((p) => p.enabled)
        .map((p) => `${encodeURIComponent(p.key)}=${encodeURIComponent(p.value)}`)
        .join('&')
      if (qs) lines.push(`  -G --data-urlencode ${shellQuote(qs)}`)
    }

    if (hasBody) {
      lines.push(`  --data-raw ${shellQuote(body)}`)
    }

    return lines.join(' \\\n')
  },

  fetch: (req, comments) => {
    const headers = applyAuthToHeaders(req.headers, req.auth)
    const lines: string[] = []

    if (comments) lines.push(`// ${req.method.toUpperCase()} ${req.url}`)
    lines.push(`const response = await fetch(${JSON.stringify(req.url)}, {`)
    lines.push(`  method: ${JSON.stringify(req.method.toUpperCase())},`)
    lines.push(`  headers: {`)
    for (const h of headers.filter((h) => h.enabled)) {
      lines.push(`    ${JSON.stringify(h.key)}: ${JSON.stringify(h.value)},`)
    }
    lines.push(`  },`)
    if (needsBody(req.method) && getBodyString(req.body)) {
      lines.push(`  body: ${JSON.stringify(getBodyString(req.body))},`)
    }
    lines.push(`})`)
    lines.push(``)
    lines.push(`const data = await response.json()`)

    return lines.join('\n')
  },

  axios: (req, comments) => {
    const headers = applyAuthToHeaders(req.headers, req.auth)
    const lines: string[] = []
    const headerObj: Record<string, string> = {}
    for (const h of headers.filter((h) => h.enabled)) {
      headerObj[h.key] = h.value
    }

    if (comments) lines.push(`// ${req.method.toUpperCase()} ${req.url}`)
    lines.push(`import axios from 'axios'`)
    lines.push(``)
    lines.push(`const response = await axios({`)
    lines.push(`  url: ${JSON.stringify(req.url)},`)
    lines.push(`  method: ${JSON.stringify(req.method.toLowerCase())},`)
    lines.push(`  headers: ${JSON.stringify(headerObj, null, 2)
      .split('\n')
      .map((l, i) => (i === 0 ? l : '  ' + l))
      .join('\n')},`)
    if (needsBody(req.method) && getBodyString(req.body)) {
      lines.push(`  data: ${JSON.stringify(getBodyString(req.body))},`)
    }
    lines.push(`})`)
    lines.push(``)
    lines.push(`console.log(response.data)`)

    return lines.join('\n')
  },

  python: (req, comments) => {
    const headers = applyAuthToHeaders(req.headers, req.auth)
    const lines: string[] = []

    if (comments) lines.push(`# ${req.method.toUpperCase()} ${req.url}`)
    lines.push(`import requests`)
    lines.push(``)

    const headerDict: Record<string, string> = {}
    for (const h of headers.filter((h) => h.enabled)) {
      headerDict[h.key] = h.value
    }

    const args: string[] = [`url=${JSON.stringify(req.url)}`]
    if (req.method.toUpperCase() !== 'GET') {
      args.push(`method=${JSON.stringify(req.method.toUpperCase())}`)
    }
    if (Object.keys(headerDict).length > 0) {
      args.push(`headers=${pyDict(headerDict)}`)
    }
    if (needsBody(req.method) && getBodyString(req.body)) {
      args.push(`data=${JSON.stringify(getBodyString(req.body))}`)
    }
    if (req.params.filter((p) => p.enabled).length > 0) {
      const params: Record<string, string> = {}
      for (const p of req.params.filter((p) => p.enabled)) {
        params[p.key] = p.value
      }
      args.push(`params=${pyDict(params)}`)
    }

    lines.push(`response = requests.request(`)
    for (const a of args) {
      lines.push(`    ${a},`)
    }
    lines.push(`)`)
    lines.push(`print(response.json())`)

    return lines.join('\n')
  },

  go: (req, comments) => {
    const headers = applyAuthToHeaders(req.headers, req.auth)
    const lines: string[] = []

    if (comments) lines.push(`// ${req.method.toUpperCase()} ${req.url}`)
    lines.push(`package main`)
    lines.push(``)
    lines.push(`import (`)
    lines.push(`\t"fmt"`)
    lines.push(`\t"io"`)
    lines.push(`\t"net/http"`)
    if (needsBody(req.method) && getBodyString(req.body)) {
      lines.push(`\t"strings"`)
    }
    lines.push(`)`)
    lines.push(``)
    lines.push(`func main() {`)
    lines.push(`\turl := ${JSON.stringify(req.url)}`)
    if (needsBody(req.method) && getBodyString(req.body)) {
      lines.push(`\tpayload := strings.NewReader(${JSON.stringify(getBodyString(req.body))})`)
      lines.push(`\treq, _ := http.NewRequest(${JSON.stringify(req.method.toUpperCase())}, url, payload)`)
    } else {
      lines.push(`\treq, _ := http.NewRequest(${JSON.stringify(req.method.toUpperCase())}, url, nil)`)
    }
    for (const h of headers.filter((h) => h.enabled)) {
      lines.push(`\treq.Header.Add(${JSON.stringify(h.key)}, ${JSON.stringify(h.value)})`)
    }
    lines.push(`\tres, err := http.DefaultClient.Do(req)`)
    lines.push(`\tif err != nil { panic(err) }`)
    lines.push(`\tdefer res.Body.Close()`)
    lines.push(`\tbody, _ := io.ReadAll(res.Body)`)
    lines.push(`\tfmt.Println(string(body))`)
    lines.push(`}`)

    return lines.join('\n')
  },

  ruby: (req, comments) => {
    const headers = applyAuthToHeaders(req.headers, req.auth)
    const lines: string[] = []

    if (comments) lines.push(`# ${req.method.toUpperCase()} ${req.url}`)
    lines.push(`require 'net/http'`)
    lines.push(`require 'uri'`)
    lines.push(`require 'json'`)
    lines.push(``)
    lines.push(`uri = URI(${JSON.stringify(req.url)})`)
    lines.push(`http = Net::HTTP.new(uri.host, uri.port)`)
    lines.push(`http.use_ssl = uri.scheme == 'https'`)
    lines.push(``)

    const method = req.method.charAt(0).toUpperCase() + req.method.slice(1).toLowerCase()
    const klass = `Net::HTTP::${method}`
    lines.push(`request = ${klass}.new(uri.request_uri)`)
    for (const h of headers.filter((h) => h.enabled)) {
      lines.push(`request[${JSON.stringify(h.key)}] = ${JSON.stringify(h.value)}`)
    }
    if (needsBody(req.method) && getBodyString(req.body)) {
      lines.push(`request.body = ${JSON.stringify(getBodyString(req.body))}`)
    }
    lines.push(``)
    lines.push(`response = http.request(request)`)
    lines.push(`puts response.body`)

    return lines.join('\n')
  },

  powershell: (req, comments) => {
    const headers = applyAuthToHeaders(req.headers, req.auth)
    const lines: string[] = []

    if (comments) lines.push(`# ${req.method.toUpperCase()} ${req.url}`)
    lines.push(`$headers = @{`)
    for (const h of headers.filter((h) => h.enabled)) {
      lines.push(`    ${psKey(h.key)} = ${JSON.stringify(h.value)}`)
    }
    lines.push(`}`)
    lines.push(``)
    lines.push(`$response = Invoke-WebRequest \\`)
    lines.push(`    -Uri ${JSON.stringify(req.url)} \\`)
    lines.push(`    -Method ${req.method.toUpperCase()} \\`)
    lines.push(`    -Headers $headers`)
    if (needsBody(req.method) && getBodyString(req.body)) {
      lines.push(`    -Body ${JSON.stringify(getBodyString(req.body))}`)
    }
    lines.push(``)
    lines.push(`$response.Content`)

    return lines.join('\n')
  },

  java: (req, comments) => {
    const headers = applyAuthToHeaders(req.headers, req.auth)
    const lines: string[] = []

    if (comments) lines.push(`// ${req.method.toUpperCase()} ${req.url}`)
    lines.push(`import java.net.URI;`)
    lines.push(`import java.net.http.HttpClient;`)
    lines.push(`import java.net.http.HttpRequest;`)
    lines.push(`import java.net.http.HttpResponse;`)
    lines.push(`import java.time.Duration;`)
    lines.push(``)
    lines.push(`public class Request {`)
    lines.push(`    public static void main(String[] args) throws Exception {`)
    lines.push(`        HttpClient client = HttpClient.newBuilder()`)
    lines.push(`            .connectTimeout(Duration.ofSeconds(30))`)
    lines.push(`            .build();`)
    lines.push(``)
    lines.push(`        HttpRequest.Builder builder = HttpRequest.newBuilder()`)
    lines.push(`            .uri(URI.create(${JSON.stringify(req.url)}))`)
    lines.push(`            .timeout(Duration.ofSeconds(30))`)

    if (req.method.toUpperCase() === 'GET') {
      lines.push(`            .GET();`)
    } else if (needsBody(req.method) && getBodyString(req.body)) {
      lines.push(
        `            .${req.method.toUpperCase()}(HttpRequest.BodyPublishers.ofString(${JSON.stringify(getBodyString(req.body))}));`
      )
    } else {
      lines.push(`            .method(${JSON.stringify(req.method.toUpperCase())}, HttpRequest.BodyPublishers.noBody());`)
    }

    for (const h of headers.filter((h) => h.enabled)) {
      lines.push(`        builder.header(${JSON.stringify(h.key)}, ${JSON.stringify(h.value)});`)
    }

    lines.push(``)
    lines.push(`        HttpResponse<String> response = client.send(builder.build(),`)
    lines.push(`            HttpResponse.BodyHandlers.ofString());`)
    lines.push(`        System.out.println(response.body());`)
    lines.push(`    }`)
    lines.push(`}`)

    return lines.join('\n')
  },
}

function pyDict(obj: Record<string, string>): string {
  const entries = Object.entries(obj)
  if (entries.length === 0) return '{}'
  const inner = entries.map(([k, v]) => `    ${JSON.stringify(k)}: ${JSON.stringify(v)},`).join('\n')
  return `{\n${inner}\n}`
}

function psKey(key: string): string {
  // PowerShell dictionary keys: quote if contains special chars
  if (/^[a-zA-Z][a-zA-Z0-9-]*$/.test(key)) return key
  return JSON.stringify(key)
}
