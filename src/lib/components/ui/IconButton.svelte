<script>
  let {
    onclick,
    title,
    ariaLabel,
    variant = "ghost",
    disabled = false,
    class: className = "",
    children,
  } = $props();

  const baseClass =
    "inline-flex items-center justify-center rounded-xl p-2 transition";

  const variants = {
    primary:
      "bg-[var(--primary)] text-[var(--primary-content)] hover:bg-[var(--primary)]/50",
    outline:
      "border border-[var(--base-300)] bg-[var(--base-100)] text-[var(--base-content)] hover:bg-[var(--base-200)]",
    ghost: "text-[var(--base-content)] hover:bg-[var(--base-200)]",
  };

  const resolvedClass = $derived(
    `${baseClass} ${variants[variant] ?? variants.ghost} ${className}`,
  );
</script>

<button
  class={resolvedClass}
  {onclick}
  {title}
  aria-label={ariaLabel ?? title}
  {disabled}
  type="button"
>
  {@render children()}
</button>
