import assert from 'node:assert/strict'
import { mkdtempSync, mkdirSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { test } from 'node:test'
import {
  generateNativeLocaleConfig,
  nativeMetadata,
  readInterfaceLanguages,
} from './native-locales.mjs'

function fixture(t, fallback, languages) {
  const root = mkdtempSync(join(tmpdir(), 'tog-locales-'))
  t.after(() => rmSync(root, { recursive: true, force: true }))
  writeFileSync(
    join(root, 'i18n.toml'),
    `fallback_language = "${fallback}"\n[fluent]\nassets_dir = "i18n"\ndomain = "interface"\n`,
  )
  for (const language of languages) {
    const directory = join(root, 'i18n', language)
    mkdirSync(directory, { recursive: true })
    writeFileSync(join(directory, 'interface.ftl'), 'hello = Hello\n')
  }
  return root
}

test('adding/removing a Fluent catalogue and changing its fallback update native declarations', (t) => {
  const root = fixture(t, 'fr', ['en', 'fr'])
  assert.deepEqual(readInterfaceLanguages(root), { fallback: 'fr', languages: ['fr', 'en'] })
  mkdirSync(join(root, 'i18n/de'))
  writeFileSync(join(root, 'i18n/de/interface.ftl'), 'hello = Hallo\n')
  // A directory without an interface catalogue is not a supported language.
  mkdirSync(join(root, 'i18n/es'))
  let metadata = nativeMetadata(readInterfaceLanguages(root))
  assert.match(metadata.plist, /CFBundleDevelopmentRegion<\/key>\s*<string>fr<\/string>/)
  assert.match(metadata.plist, /<string>de<\/string>/)
  assert.doesNotMatch(metadata.plist, /<string>es<\/string>/)
  assert.deepEqual(metadata.windows.nsis.languages, ['French', 'German', 'English'])
  assert.deepEqual(metadata.windows.wix.language, ['fr-FR', 'de-DE', 'en-US'])
  rmSync(join(root, 'i18n/en'), { recursive: true })
  metadata = nativeMetadata(readInterfaceLanguages(root))
  assert.deepEqual(metadata.windows.nsis.languages, ['French', 'German'])
  assert.doesNotMatch(metadata.plist, /<string>en<\/string>/)
})

test('a missing fallback catalogue or unsupported native identifier fails explicitly', (t) => {
  const root = fixture(t, 'fr', ['en'])
  assert.throws(() => readInterfaceLanguages(root), /Catalogue de repli absent/)
  assert.throws(
    () => nativeMetadata({ fallback: 'ja', languages: ['ja'] }),
    /Identifiants d’installateur à définir/,
  )
})

test('development and packaging share the conventional generated Info.plist', (t) => {
  const root = fixture(t, 'fr', ['en', 'fr'])
  const project = join(root, 'application')
  const config = generateNativeLocaleConfig(project, root)
  const plist = join(project, 'src-tauri/Info.plist')
  assert.equal(readFileSync(plist, 'utf8'), nativeMetadata(readInterfaceLanguages(root)).plist)
  // A custom infoPlist override alone would pass a bundle check but break dev.
  const overrides = JSON.parse(readFileSync(config, 'utf8'))
  assert.equal(overrides.bundle.macOS, undefined)
  assert.deepEqual(overrides.bundle.windows.nsis.languages, ['French', 'English'])
  const modified = statSync(plist).mtimeMs
  generateNativeLocaleConfig(project, root)
  assert.equal(statSync(plist).mtimeMs, modified, 'do not force recompilation on every launch')
  writeFileSync(
    join(root, 'i18n.toml'),
    'fallback_language = "en"\n[fluent]\nassets_dir = "i18n"\ndomain = "interface"\n',
  )
  generateNativeLocaleConfig(project, root)
  assert.equal(readFileSync(plist, 'utf8'), nativeMetadata(readInterfaceLanguages(root)).plist)
})
