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
      "bg-primary text-primary-content hover:bg-primary/80",
    outline:
      "border border-base-300 bg-base-100 text-base-content hover:bg-base-300",
    ghost: "text-base-content hover:bg-base-200",
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
