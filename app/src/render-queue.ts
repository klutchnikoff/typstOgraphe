/** Latest-only queue: no typing debounce, no parallel compiles, no backlog. */
export function renderQueue(render: () => Promise<void>) {
  let pending = false,
    queued = false,
    running = false,
    paused = false
  const idle: (() => void)[] = []
  const drain = () => {
    if (paused || queued || running || !pending) return
    queued = true
    queueMicrotask(async () => {
      queued = false
      if (paused || !pending) return
      running = true
      pending = false
      try {
        await render()
      } finally {
        running = false
        idle.splice(0).forEach((resolve) => resolve())
        drain()
      }
    })
  }
  return {
    request: () => {
      pending = true
      drain()
    },
    pause: async () => {
      paused = true
      if (running) await new Promise<void>((resolve) => idle.push(resolve))
    },
    resume: () => {
      paused = false
      drain()
    },
  }
}
