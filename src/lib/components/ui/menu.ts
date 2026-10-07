export type MenuItem = {
  label: string;
  danger?: boolean;
  disabled?: boolean;
  onSelect: () => void;
};

export const menuContentClass =
  "border-base-300 bg-base-100 z-(--z-popover) min-w-28 rounded-xl border p-1 shadow-lg outline-none";

export const menuItemClass = (danger = false) =>
  `block w-full cursor-default rounded-lg px-2.5 py-1.5 text-left text-[13px] whitespace-nowrap outline-none transition select-none data-disabled:opacity-40 ${
    danger
      ? "text-error data-highlighted:bg-error/10"
      : "text-base-content data-highlighted:bg-base-200"
  }`;
