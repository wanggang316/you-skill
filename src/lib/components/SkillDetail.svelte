<script>
  import { onMount } from 'svelte'
  import { Loader2, FileText } from '@lucide/svelte'
  import { parseMarkdown, renderMarkdownBody } from '../utils/markdown'
  import { t } from '../i18n'
  import { api } from '../api/skills'

  let {
    skill,
    type,
    agents
  } = $props()

  let content = $state('')
  let parsedFrontmatter = $state({})
  let hasFrontmatter = $state(false)
  let loading = $state(true)
  let error = $state('')

  // Convert GitHub URL to raw content URL
  function convertToRawUrl(url, path) {
    if (!url || !path) return null

    // Handle github.com URLs
    if (url.includes('github.com')) {
      // Extract owner and repo from URL like https://github.com/owner/repo
      const match = url.match(/github\.com\/([^\/]+)\/([^\/]+)/)
      if (match) {
        const [, owner, repo] = match
        // Remove .git suffix if present
        const cleanRepo = repo.replace(/\.git$/, '')
        // Construct raw URL: https://raw.githubusercontent.com/owner/repo/refs/heads/main/path/SKILL.md
        return `https://raw.githubusercontent.com/${owner}/${cleanRepo}/refs/heads/main/${path}/SKILL.md`
      }
    }
    return null
  }

  async function loadLocalSkillContent() {
    if (!skill.canonical_path) {
      throw new Error('Skill path is missing')
    }

    console.log('[SkillDetail] Reading skill readme from:', skill.canonical_path)
    // Use backend API to read file (bypasses frontend FS restrictions)
    return await api.readSkillReadme(skill.canonical_path)
  }

  async function loadRemoteSkillContent() {
    const rawUrl = convertToRawUrl(skill.url, skill.path)
    if (!rawUrl) {
      throw new Error('Unable to construct raw URL for this skill')
    }

    const response = await fetch(rawUrl)
    if (!response.ok) {
      throw new Error(`Failed to fetch: ${response.status} ${response.statusText}`)
    }
    return await response.text()
  }

  async function loadContent() {
    loading = true
    error = ''
    try {
      let rawContent = ''
      if (type === 'local') {
        rawContent = await loadLocalSkillContent()
      } else {
        rawContent = await loadRemoteSkillContent()
      }

      // Parse frontmatter and body content
      const parsed = parseMarkdown(rawContent)
      parsedFrontmatter = parsed.frontmatter
      hasFrontmatter = parsed.hasFrontmatter
      content = parsed.content
    } catch (e) {
      error = String(e)
      content = ''
      parsedFrontmatter = {}
      hasFrontmatter = false
    } finally {
      loading = false
    }
  }

  function markdownInteractions(node) {
    const handleClick = async (event) => {
      const button = event.target.closest('.markdown-code-block__copy')
      if (button) {
        event.preventDefault()
        event.stopPropagation()

        const block = button.closest('.markdown-code-block')
        const codeElement = block?.querySelector('code')
        const codeContent = codeElement?.textContent ?? ''

        if (!codeContent) return

        try {
          await navigator.clipboard.writeText(codeContent)
        } catch (err) {
          // Fallback copy
          const textarea = document.createElement('textarea')
          textarea.value = codeContent
          textarea.setAttribute('readonly', '')
          textarea.style.position = 'absolute'
          textarea.style.left = '-9999px'
          document.body.appendChild(textarea)
          textarea.select()
          try {
            document.execCommand('copy')
          } catch {}
          document.body.removeChild(textarea)
        }

        button.classList.add('copied')
        const timerId = button.dataset.copyTimeout
        if (timerId) {
          window.clearTimeout(Number(timerId))
        }
        const timeoutHandle = window.setTimeout(() => {
          button.classList.remove('copied')
          delete button.dataset.copyTimeout
        }, 1500)
        button.dataset.copyTimeout = String(timeoutHandle)
        return
      }

      const link = event.target.closest('a[href]')
      if (link) {
        const href = link.getAttribute('href')
        if (href?.startsWith('http://') || href?.startsWith('https://')) {
          event.preventDefault()
          event.stopPropagation()
          const { open } = await import('@tauri-apps/plugin-shell')
          await open(href)
        }
      }
    }

    node.addEventListener('click', handleClick)

    return {
      destroy() {
        node.removeEventListener('click', handleClick)
      },
    }
  }

  onMount(() => {
    loadContent()
  })

  // Reload when skill changes
  $effect(() => {
    if (skill) {
      loadContent()
    }
  })
</script>

<div class="max-w-4xl mx-auto">
  {#if loading}
    <div class="flex items-center justify-center py-12">
      <Loader2 size={32} class="animate-spin text-[var(--base-content-muted)]" />
    </div>
  {:else if error}
    <div class="rounded-2xl border border-[var(--error-border)] bg-[var(--base-100)] p-6 text-center">
      <p class="text-[var(--error)]">{error}</p>
      <button
        class="mt-4 rounded-lg bg-[var(--primary)] px-4 py-2 text-sm text-[var(--primary-content)] transition hover:opacity-90"
        onclick={loadContent}
      >
        {$t('detail.retry')}
      </button>
    </div>
  {:else}
    <!-- Frontmatter Card -->
    {#if hasFrontmatter}
      <div class="frontmatter-card">
        <div class="frontmatter-header">
          <FileText size={18} />
          <span>Skill Metadata</span>
        </div>
        <div class="frontmatter-content">
          {#if parsedFrontmatter.name}
            <div class="frontmatter-item">
              <span class="frontmatter-key">Name</span>
              <span class="frontmatter-value">{parsedFrontmatter.name}</span>
            </div>
          {/if}
          {#if parsedFrontmatter.description}
            <div class="frontmatter-item">
              <span class="frontmatter-key">Description</span>
              <span class="frontmatter-value">{parsedFrontmatter.description}</span>
            </div>
          {/if}
        </div>
      </div>
    {/if}

    <div class="markdown-content" use:markdownInteractions>
      {@html renderMarkdownBody(content)}
    </div>
  {/if}
</div>

<style>
  :global(.markdown-content) {
    line-height: 1.8;
    color: var(--base-content);
  }

  :global(.markdown-content h1) {
    font-size: 1.5rem;
    font-weight: 700;
    margin: 1.5rem 0 1rem 0;
    color: var(--base-content);
    border-bottom: 2px solid var(--base-300);
    padding-bottom: 0.5rem;
  }

  :global(.markdown-content h2) {
    font-size: 1.25rem;
    font-weight: 600;
    margin: 1.25rem 0 0.75rem 0;
    color: var(--base-content);
    border-bottom: 1px solid var(--base-300);
    padding-bottom: 0.25rem;
  }

  :global(.markdown-content h3) {
    font-size: 1.125rem;
    font-weight: 600;
    margin: 1rem 0 0.5rem 0;
    color: var(--base-content);
  }

  :global(.markdown-content h4) {
    font-size: 1rem;
    font-weight: 600;
    margin: 0.75rem 0 0.5rem 0;
    color: var(--base-content);
  }

  :global(.markdown-content h5) {
    font-size: 0.875rem;
    font-weight: 600;
    margin: 0.5rem 0 0.25rem 0;
    color: var(--base-content);
  }

  :global(.markdown-content h6) {
    font-size: 0.75rem;
    font-weight: 600;
    margin: 0.5rem 0 0.25rem 0;
    color: var(--base-content-muted);
  }

  :global(.markdown-content p) {
    margin: 0.5rem 0;
    line-height: 1.8;
    font-weight: 400;
  }

  :global(.markdown-content strong) {
    font-weight: 600;
    color: var(--base-content);
  }

  :global(.markdown-content em) {
    font-style: italic;
    color: var(--base-content);
  }

  :global(.markdown-content code:not(.hljs)) {
    background-color: var(--base-300);
    color: var(--base-content);
    padding: 0.125rem 0.25rem;
    border-radius: 0.25rem;
    font-family: "Fira Code", "Monaco", "Consolas", "Courier New", monospace;
    font-size: 0.875em;
  }

  :global(.markdown-content pre) {
    margin: 1rem 0;
    overflow-x: auto;
  }

  :global(.markdown-content pre code) {
    font-family: "Fira Code", "Monaco", "Consolas", "Courier New", monospace;
    font-size: 0.8rem;
    line-height: 1.5;
  }

  :global(.markdown-content .markdown-code-block) {
    margin: 1.25rem 0;
    border-radius: 0.75rem;
    border: 1px solid var(--base-300);
    background-color: var(--base-200);
    overflow: hidden;
  }

  :global(.markdown-content .markdown-code-block__header) {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    padding: 0.3rem 0.9rem;
    background-color: rgba(148, 163, 184, 0.12);
    border-bottom: 1px solid rgba(148, 163, 184, 0.2);
    font-size: 0.75rem;
    line-height: 1;
    color: var(--base-content-muted);
    text-transform: lowercase;
  }

  :global(.markdown-content .markdown-code-block__language) {
    font-family: "Fira Code", "Monaco", "Consolas", "Courier New", monospace;
  }

  :global(.markdown-content .markdown-code-block__copy) {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 1.75rem;
    height: 1.75rem;
    border-radius: 0.5rem;
    border: none;
    background: transparent;
    color: inherit;
    cursor: pointer;
  }

  :global(.markdown-content .markdown-code-block__copy:hover) {
    background-color: var(--base-300);
  }

  :global(.markdown-content .markdown-code-block__copy-icon) {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 1rem;
    height: 1rem;
    pointer-events: none;
  }

  :global(.markdown-content .markdown-code-block__copy-label) {
    display: none;
    font-size: 0.7rem;
    pointer-events: none;
  }

  :global(.markdown-content .markdown-code-block__copy.copied) {
    background-color: transparent;
    color: var(--success);
  }

  :global(.markdown-content .markdown-code-block__copy.copied .markdown-code-block__copy-icon) {
    display: none;
  }

  :global(.markdown-content .markdown-code-block__copy.copied .markdown-code-block__copy-label) {
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  :global(.markdown-content .markdown-code-block pre) {
    margin: 0;
    background: transparent;
  }

  :global(.markdown-content .markdown-code-block pre code) {
    background: transparent;
    display: block;
    padding: 1.1rem 1.25rem;
  }

  :global(.markdown-content ul),
  :global(.markdown-content ol) {
    margin: 0.75rem 0;
    padding-left: 1.5rem;
  }

  :global(.markdown-content li) {
    margin: 0.25rem 0;
    line-height: 1.6;
  }

  :global(.markdown-content blockquote) {
    border-left: 4px solid var(--primary);
    background-color: var(--base-200);
    margin: 1rem 0;
    padding: 0.75rem 1rem;
    border-radius: 0 0.25rem 0.25rem 0;
    color: var(--base-content-muted);
    font-style: italic;
  }

  :global(.markdown-content a) {
    color: var(--primary);
    text-decoration: underline;
    text-decoration-color: transparent;
    transition: text-decoration-color 0.2s;
  }

  :global(.markdown-content a:hover) {
    text-decoration-color: var(--primary);
  }

  :global(.markdown-content hr) {
    border: none;
    border-top: 1px solid var(--base-300);
    margin: 2rem 0;
  }

  :global(.markdown-content table) {
    width: 100%;
    border-collapse: collapse;
    margin: 1rem 0;
    border: 1px solid var(--base-300);
    border-radius: 0.5rem;
    overflow: hidden;
  }

  :global(.markdown-content th),
  :global(.markdown-content td) {
    padding: 0.75rem;
    text-align: left;
    border-bottom: 1px solid var(--base-300);
  }

  :global(.markdown-content th) {
    background-color: var(--base-200);
    font-weight: 600;
    color: var(--base-content);
  }

  :global(.markdown-content tr:last-child td) {
    border-bottom: none;
  }

  :global(.markdown-content img) {
    max-width: 100%;
    height: auto;
    border-radius: 0.5rem;
  }

  /* Frontmatter Card Styles */
  .frontmatter-card {
    margin-bottom: 1.5rem;
    border-radius: 0.75rem;
    border: 1px solid var(--base-300);
    background: var(--base-200);
    overflow: hidden;
  }

  .frontmatter-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.75rem 1rem;
    background: var(--base-300);
    color: var(--base-content-muted);
    font-size: 0.875rem;
    font-weight: 500;
  }

  .frontmatter-content {
    padding: 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .frontmatter-item {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .frontmatter-key {
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--base-content-muted);
    text-transform: uppercase;
    letter-spacing: 0.025em;
  }

  .frontmatter-value {
    font-size: 0.875rem;
    color: var(--base-content);
    line-height: 1.5;
  }

  .frontmatter-item:first-child .frontmatter-value {
    font-size: 1rem;
    font-weight: 600;
  }
</style>
