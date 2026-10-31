import type { Plugin } from 'vite'

/** Glob imports outside Vite's root also need directory watches for add/unlink. */
export function watchExternalAssets(...directories: string[]): Plugin {
  return {
    name: 'watch-external-assets',
    configureServer(server) {
      server.watcher.add(directories)
    },
  }
}
