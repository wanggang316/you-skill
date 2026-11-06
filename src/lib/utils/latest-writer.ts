/**
 * Runs `write` one call at a time and always with the latest value. A value that arrives
 * while a write runs replaces any value that still waits, so overlapping saves cannot
 * finish out of order and leave an older value stored.
 */
export function createLatestWriter<T>(write: (value: T) => Promise<unknown>) {
  let waiting: { value: T } | null = null;
  let running: Promise<void> | null = null;

  async function drain(): Promise<void> {
    while (waiting) {
      const { value } = waiting;
      waiting = null;
      try {
        await write(value);
      } catch (error) {
        console.error(error);
      }
    }
    running = null;
  }

  /** Resolves when the value, or a later one, is written. */
  return (value: T): Promise<void> => {
    waiting = { value };
    running ??= drain();
    return running;
  };
}
