import assert from 'node:assert/strict'
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { test } from 'node:test'
import { nativeMetadata, readInterfaceLanguages } from './native-locales.mjs'

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
