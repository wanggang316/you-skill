<script>
  let {
    onclick,
    title,
    ariaLabel,
    variant = 'ghost',
    disabled = false,
    class: className = ''
  } = $props()

  const baseClass =
    'inline-flex items-center justify-center rounded-xl p-2 transition'

  const variants = {
    primary: 'bg-[var(--primary)] text-[var(--primary-content)]',
    outline:
      'border border-[var(--base-300)] bg-[var(--base-100)] text-[var(--base-content)]',
    ghost: 'text-[var(--base-content)]'
  }

  const resolvedClass = `${baseClass} ${variants[variant] ?? variants.ghost} ${className}`
</script>

<button
  class={resolvedClass}
  onclick={onclick}
  title={title}
  aria-label={ariaLabel ?? title}
  {disabled}
  type="button"
>
  <slot />
</button>
