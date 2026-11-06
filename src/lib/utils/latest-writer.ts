type Waiter = { resolve: () => void; reject: (error: unknown) => void };

/**
 * Runs `write` one call at a time and always with the latest value. A value that arrives
 * while a write runs replaces any value that still waits, so overlapping saves cannot
 * finish out of order and leave an older value stored. Each call settles with the write
 * that stored its value or a later one, and rejects when that write fails.
 */
export function createLatestWriter<T>(write: (value: T) => Promise<unknown>) {
  let waiting: { value: T; waiters: Waiter[] } | null = null;
  let running = false;

  async function drain(): Promise<void> {
    running = true;
    while (waiting) {
      const { value, waiters } = waiting;
      waiting = null;
      try {
        await write(value);
        waiters.forEach((waiter) => waiter.resolve());
      } catch (error) {
        waiters.forEach((waiter) => waiter.reject(error));
      }
    }
    running = false;
  }

  return (value: T): Promise<void> =>
    new Promise<void>((resolve, reject) => {
      // A replaced value is stored by the write of the newer one, so its caller waits for it.
      const waiters = waiting?.waiters ?? [];
      waiters.push({ resolve, reject });
      waiting = { value, waiters };
      if (!running) void drain();
    });
}
