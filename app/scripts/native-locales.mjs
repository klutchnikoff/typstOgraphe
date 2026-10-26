import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs'
import { join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = fileURLToPath(new URL('../', import.meta.url))

// Platform identifiers, not an availability list: only existing Fluent catalogues
// are advertised. A new language may require an installer identifier here.
const installerLanguages = {
  en: { nsis: 'English', wix: 'en-US' },
  fr: { nsis: 'French', wix: 'fr-FR' },
  de: { nsis: 'German', wix: 'de-DE' },
  es: { nsis: 'Spanish', wix: 'es-ES' },
  it: { nsis: 'Italian', wix: 'it-IT' },
  pt: { nsis: 'Portuguese', wix: 'pt-PT' },
  nl: { nsis: 'Dutch', wix: 'nl-NL' },
}

export function readInterfaceLanguages(directory) {
  const config = readFileSync(join(directory, 'i18n.toml'), 'utf8')
  // These scalar fields come from i18n-embed's configuration; reject unsupported
  // spellings rather than silently inventing a fallback or a catalogue directory.
  const scalar = (key) => {
    const match = config.match(
      new RegExp(`^${key}\\s*=\\s*(["'])([^"'\\r\\n]+)\\1\\s*(?:#.*)?$`, 'm'),
    )
    if (!match) throw new Error(`Champ ${key} absent ou non pris en charge dans i18n.toml`)
    return match[2]
  }
  const fallback = scalar('fallback_language')
  const assets = resolve(directory, scalar('assets_dir'))
  const domain = scalar('domain')
  const languages = readdirSync(assets, { withFileTypes: true })
    .filter((entry) => entry.isDirectory() && existsSync(join(assets, entry.name, `${domain}.ftl`)))
    .map((entry) => {
      if (new Intl.Locale(entry.name).toString() !== entry.name)
        throw new Error(`Identifiant de catalogue non canonique : ${entry.name}`)
      return entry.name
    })
    .sort()
  if (!languages.includes(fallback)) throw new Error(`Catalogue de repli absent : ${fallback}`)
  return {
    fallback,
    languages: [fallback, ...languages.filter((language) => language !== fallback)],
  }
}

export function nativeMetadata({ fallback, languages }) {
  const installers = languages.map((language) => {
    const identifiers = installerLanguages[language]
    if (!identifiers)
      throw new Error(
        `Identifiants d’installateur à définir pour le catalogue Fluent : ${language}`,
      )
    return identifiers
  })
  return {
    plist: `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>${fallback}</string>
    <key>CFBundleLocalizations</key>
    <array>
${languages.map((language) => `        <string>${language}</string>`).join('\n')}
    </array>
</dict>
</plist>
`,
    windows: {
      nsis: { languages: installers.map((language) => language.nsis) },
      wix: { language: installers.map((language) => language.wix) },
    },
  }
}

export function generateNativeLocaleConfig(
  projectRoot = root,
  catalogueRoot = resolve(projectRoot, '../crates/i18n'),
) {
  const source = readInterfaceLanguages(catalogueRoot)
  const metadata = nativeMetadata(source)
  const directory = join(projectRoot, 'src-tauri/gen/native-locales')
  mkdirSync(directory, { recursive: true })
  const writeIfChanged = (path, value) => {
    if (!existsSync(path) || readFileSync(path, 'utf8') !== value) writeFileSync(path, value)
  }
  // Tauri's development executable embeds only this conventional path.
  // bundle.macOS.infoPlist works for packaging, but is ignored by dev codegen.
  // Both launches must therefore use this same generated file.
  const plist = join(projectRoot, 'src-tauri/Info.plist')
  writeIfChanged(plist, metadata.plist)
  const path = join(directory, 'tauri.locales.json')
  writeIfChanged(path, JSON.stringify({ bundle: { windows: metadata.windows } }, null, 2) + '\n')
  return path
}

// Direct Cargo launches use the same generator as the Tauri CLI wrapper.
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url))
  generateNativeLocaleConfig()
