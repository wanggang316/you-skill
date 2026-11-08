import { tick } from "svelte";

/**
 * Decides whether an open change of a collapsible group animates. Only the change that a
 * header click causes animates. Any other change (first render, search, selection, clearing a
 * search) is instant, also while a click transition is still running.
 */
export class ClickMotion {
  /** The open state that the last header click requested, until its transition ends. */
  #target = $state<boolean | null>(null);
  #isOpen: () => boolean;
  #settle: () => Promise<void>;

  constructor(isOpen: () => boolean, settle: () => Promise<void> = tick) {
    this.#isOpen = isOpen;
    this.#settle = settle;
  }

  get animate() {
    return this.#target !== null && this.#isOpen() === this.#target;
  }

  async click(toggle: () => void) {
    const before = this.#isOpen();
    this.#target = !before;
    toggle();
    await this.#settle();
    // The parent can keep the state, for example a search that forces the group open. The
    // request must not carry over to a later change that no click caused.
    if (this.#isOpen() === before) this.#target = null;
  }

  /**
   * Call when an open or close transition starts. Returns whether it may animate. A transition
   * that the click did not cause ends the click intent, so a later return to the clicked state
   * (for example, a search cleared during that transition) stays instant.
   */
  start() {
    const animate = this.animate;
    if (!animate) this.#target = null;
    return animate;
  }

  /** Call when the open or close transition ends. */
  done() {
    this.#target = null;
  }
}
