import { run } from '@tauri-apps/cli'
import { generateNativeLocaleConfig } from './native-locales.mjs'

const args = process.argv.slice(2)
// Tauri must read the generated overrides before parsing build/bundle settings.
// A beforeBuildCommand would run too late; info/init and other commands need no overlay.
if (['dev', 'build', 'bundle'].includes(args[0]))
  args.splice(1, 0, '--config', generateNativeLocaleConfig())
await run(args)
