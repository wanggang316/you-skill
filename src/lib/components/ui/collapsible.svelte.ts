import { tick } from "svelte";

/**
 * Decides whether the next open change of a collapsible group animates. Only a header click
 * that changes the visible open state animates. Any other change (first render, search,
 * selection, clearing a search) is instant.
 */
export class ClickMotion {
  animate = $state(false);
  #isOpen: () => boolean;
  #settle: () => Promise<void>;

  constructor(isOpen: () => boolean, settle: () => Promise<void> = tick) {
    this.#isOpen = isOpen;
    this.#settle = settle;
  }

  async click(toggle: () => void) {
    const before = this.#isOpen();
    this.animate = true;
    toggle();
    await this.#settle();
    // The parent can keep the state, for example a search that forces the group open. The
    // request must not carry over to a later change that no click caused.
    if (this.#isOpen() === before) this.animate = false;
  }

  /** Call when the open or close transition ends. */
  done() {
    this.animate = false;
  }
}
