import { Marked } from 'marked'
import { markedHighlight } from 'marked-highlight'
import type { Tokens } from 'marked'
import hljs from 'highlight.js/lib/common'
import matter from 'gray-matter'

const CODE_BLOCK_LANG_PATTERN = /\s+/

const LANGUAGE_LABEL_MAP: Record<string, string> = {
  bash: 'Bash',
  shell: 'Shell',
  sh: 'Shell',
  c: 'C',
  cpp: 'C++',
  cs: 'C#',
  css: 'CSS',
  docker: 'Docker',
  go: 'Go',
  golang: 'Go',
  html: 'HTML',
  java: 'Java',
  javascript: 'JavaScript',
  js: 'JavaScript',
  json: 'JSON',
  jsx: 'JSX',
  kotlin: 'Kotlin',
  lua: 'Lua',
  markdown: 'Markdown',
  md: 'Markdown',
  php: 'PHP',
  plaintext: 'Text',
  python: 'Python',
  py: 'Python',
  ruby: 'Ruby',
  rust: 'Rust',
  sql: 'SQL',
  swift: 'Swift',
  text: 'Text',
  toml: 'TOML',
  ts: 'TypeScript',
  tsx: 'TSX',
  typescript: 'TypeScript',
  vue: 'Vue',
  xml: 'XML',
  yaml: 'YAML',
  yml: 'YAML',
}

const COPY_BUTTON_ICON = `<svg class="code-icon" xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>`
const COPY_BUTTON_LABEL = 'Copied'

function escapeHtml(value: string): string {
  return value
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
}

function resolveLanguage(language?: string | null): string | undefined {
  if (!language) return undefined
  const normalized = language.trim().toLowerCase()
  if (!normalized) return undefined
  const [candidate] = normalized.split(CODE_BLOCK_LANG_PATTERN)
  return candidate || undefined
}

function formatLanguageLabel(language?: string): string {
  if (!language) return 'Text'
  const mapped = LANGUAGE_LABEL_MAP[language]
  if (mapped) return mapped
  return (
    language
      .split(/[-_]/)
      .filter(Boolean)
      .map((segment) => segment.charAt(0).toUpperCase() + segment.slice(1))
      .join(' ') || language.toUpperCase()
  )
}

const markedRenderer = new Marked(
  markedHighlight({
    langPrefix: 'hljs language-',
    highlight(code, language) {
      const resolvedLanguage = resolveLanguage(language)

      if (resolvedLanguage && hljs.getLanguage(resolvedLanguage)) {
        try {
          return hljs.highlight(code, { language: resolvedLanguage }).value
        } catch (error) {
          console.warn('highlight.js failed', {
            language: resolvedLanguage,
            error,
          })
        }
      }

      try {
        return hljs.highlightAuto(code).value
      } catch (error) {
        console.warn('highlight.js auto detection failed', { error })
        return escapeHtml(code)
      }
    },
  }),
)

markedRenderer.use({
  renderer: {
    code(token: Tokens.Code) {
      const language = resolveLanguage(token.lang ?? undefined)
      const languageLabel = formatLanguageLabel(language)
      const escapedLabel = escapeHtml(languageLabel)
      const languageAttr = language
        ? ` data-language="${escapeHtml(language)}"`
        : ''
      const classNames = ['hljs']
      if (language) {
        classNames.push(`language-${language}`)
      }

      const codeHtml = token.escaped ? token.text : escapeHtml(token.text)

      return `<figure class="markdown-code-block"${languageAttr}>
  <header class="markdown-code-block__header">
    <span class="markdown-code-block__language">${escapedLabel}</span>
    <button type="button" class="markdown-code-block__copy" aria-label="Copy code">
      <span class="markdown-code-block__copy-icon">${COPY_BUTTON_ICON}</span>
      <span class="markdown-code-block__copy-label">${COPY_BUTTON_LABEL}</span>
    </button>
  </header>
  <pre><code class="${classNames.join(' ')}">${codeHtml}\n</code></pre>
</figure>`
    },
    image({ href, title, text }: Tokens.Image) {
      const src = href ?? ''
      const alt = escapeHtml(text ?? '')
      const titleAttr = title ? ` title="${escapeHtml(title)}"` : ''
      return `<img src="${escapeHtml(src)}" alt="${alt}"${titleAttr} />`
    },
  },
})

markedRenderer.options({ async: false })

export function renderMarkdown(content: string | undefined | null): string {
  if (!content) return ''

  const result = markedRenderer.parse(content)
  return typeof result === 'string' ? result : ''
}

// Parsed frontmatter data structure
export interface ParsedFrontmatter {
  name?: string
  description?: string
  [key: string]: unknown
}

// Result of parsing markdown with frontmatter
export interface ParsedMarkdown {
  frontmatter: ParsedFrontmatter
  content: string
  hasFrontmatter: boolean
}

/**
 * Parse markdown content and extract frontmatter
 * Returns structured data with frontmatter and body content
 */
export function parseMarkdown(content: string | undefined | null): ParsedMarkdown {
  if (!content) {
    return {
      frontmatter: {},
      content: '',
      hasFrontmatter: false
    }
  }

  const parsed = matter(content)

  return {
    frontmatter: parsed.data as ParsedFrontmatter,
    content: parsed.content.trim(),
    hasFrontmatter: parsed.isEmpty === false && Object.keys(parsed.data).length > 0
  }
}

/**
 * Render markdown body only (without frontmatter)
 */
export function renderMarkdownBody(content: string | undefined | null): string {
  const { content: body } = parseMarkdown(content)
  return renderMarkdown(body)
}

/**
 * Format frontmatter value for display
 */
export function formatFrontmatterValue(value: unknown): string {
  if (value === null || value === undefined) return ''
  if (typeof value === 'boolean') return value ? 'Yes' : 'No'
  if (typeof value === 'string') return value
  if (Array.isArray(value)) return value.join(', ')
  if (typeof value === 'object') return JSON.stringify(value)
  return String(value)
}
