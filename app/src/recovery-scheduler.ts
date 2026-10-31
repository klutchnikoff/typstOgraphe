/** Trailing pause plus a fixed maximum deadline. Writes are serialized and retried on failure. */
export class RecoveryScheduler {
  private pause?: ReturnType<typeof setTimeout>
  private deadline?: ReturnType<typeof setTimeout>
  private pending?: Promise<void>
  private dirty = false
  private stopped = false
  private reported = false
  constructor(
    private write: () => Promise<void>,
    private failed: (error: unknown) => void,
    private delay = 2000,
    private maximum = 10000,
  ) {}
  changed = () => {
    if (this.stopped) return
    this.dirty = true
    clearTimeout(this.pause)
    this.pause = setTimeout(this.run, this.delay)
    this.deadline ??= setTimeout(this.run, this.maximum)
  }
  private clear() {
    clearTimeout(this.pause)
    clearTimeout(this.deadline)
    this.pause = this.deadline = undefined
  }
  private run = () => {
    void this.flush().catch((error) => {
      if (!this.reported) {
        this.reported = true
        this.failed(error)
      }
    })
  }
  async flush(): Promise<void> {
    if (this.stopped) return
    if (this.pending) {
      await this.pending
      return this.flush()
    }
    if (!this.dirty) return
    this.clear()
    this.dirty = false
    this.pending = this.write()
      .then(() => {
        this.reported = false
      })
      .catch((error) => {
        if (!this.stopped) this.changed()
        throw error
      })
      .finally(() => {
        this.pending = undefined
      })
    await this.pending
    // A lifecycle flush also protects edits made while the write was pending.
    return this.flush()
  }
  stop() {
    this.stopped = true
    this.clear()
  }
}
