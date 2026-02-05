<script>
  let language = $state('en')
  let theme = $state('system')
  let syncMode = $state('symlink')
  let checkingUpdate = $state(false)
  let updateMessage = $state('')

  const handleCheckUpdate = async () => {
    checkingUpdate = true
    updateMessage = ''
    try {
      await new Promise((resolve) => setTimeout(resolve, 300))
      updateMessage = '暂未接入更新服务'
    } finally {
      checkingUpdate = false
    }
  }
</script>

<section class="space-y-6">
  <div class="rounded-2xl border border-[var(--base-300)] bg-[var(--base-100)] p-5">
    <div class="flex items-center justify-between">
      <div>
        <p class="text-sm font-semibold">通用</p>
        <p class="text-xs text-[var(--base-content-muted)]">
          语言、主题与同步方式
        </p>
      </div>
    </div>
    <div class="mt-4 grid gap-4 md:grid-cols-3">
      <label class="space-y-2 text-sm text-[var(--base-content)]">
        <span class="text-xs text-[var(--base-content-muted)]">语言</span>
        <select
          class="w-full rounded-xl border border-[var(--base-300)] bg-[var(--base-100)] px-3 py-2"
          bind:value={language}
        >
          <option value="en">English</option>
          <option value="zh">中文</option>
        </select>
      </label>
      <label class="space-y-2 text-sm text-[var(--base-content)]">
        <span class="text-xs text-[var(--base-content-muted)]">主题</span>
        <select
          class="w-full rounded-xl border border-[var(--base-300)] bg-[var(--base-100)] px-3 py-2"
          bind:value={theme}
        >
          <option value="system">跟随主题</option>
          <option value="light">浅色</option>
          <option value="dark">深色</option>
        </select>
      </label>
      <label class="space-y-2 text-sm text-[var(--base-content)]">
        <span class="text-xs text-[var(--base-content-muted)]">
          Skill 同步方式
        </span>
        <select
          class="w-full rounded-xl border border-[var(--base-300)] bg-[var(--base-100)] px-3 py-2"
          bind:value={syncMode}
        >
          <option value="symlink">软连接</option>
          <option value="copy">复制</option>
        </select>
      </label>
    </div>
  </div>

  <div class="rounded-2xl border border-[var(--base-300)] bg-[var(--base-100)] p-5">
    <div>
      <p class="text-sm font-semibold">关于</p>
      <p class="text-xs text-[var(--base-content-muted)]">版本与更新</p>
    </div>
    <div class="mt-4 flex flex-wrap items-center gap-3">
      <button
        class="rounded-xl bg-[var(--primary)] px-4 py-2 text-sm text-[var(--primary-content)]"
        onclick={handleCheckUpdate}
        disabled={checkingUpdate}
        type="button"
      >
        {checkingUpdate ? '检查中...' : '检查更新'}
      </button>
      {#if updateMessage}
        <span class="text-sm text-[var(--base-content-muted)]">
          {updateMessage}
        </span>
      {/if}
    </div>
  </div>
</section>
