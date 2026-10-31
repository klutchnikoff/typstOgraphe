import assert from 'node:assert/strict'
import { execFileSync } from 'node:child_process'
import { closeSync, openSync, readSync } from 'node:fs'
import { resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { readInterfaceLanguages } from './native-locales.mjs'

const catalogue = fileURLToPath(new URL('../../crates/i18n/', import.meta.url))

export function embeddedPlist(binary) {
  const commands = execFileSync('/usr/bin/otool', ['-l', binary], { encoding: 'utf8' })
  const sections = [
    ...commands.matchAll(
      /sectname __info_plist\s+segname __TEXT\s+addr \S+\s+size (0x[\da-f]+)\s+offset (\d+)/g,
    ),
  ]
  assert.equal(sections.length, 1, `${binary}: expected exactly one embedded Info.plist`)
  const size = Number.parseInt(sections[0][1], 16)
  const offset = Number(sections[0][2])
  assert.ok(size > 0 && size <= 64 * 1024, 'invalid embedded Info.plist size')
  const fd = openSync(binary, 'r')
  const bytes = Buffer.alloc(size)
  try {
    assert.equal(readSync(fd, bytes, 0, size, offset), size, 'truncated embedded Info.plist')
  } finally {
    closeSync(fd)
  }
  return JSON.parse(
    execFileSync('/usr/bin/plutil', ['-convert', 'json', '-o', '-', '-'], { input: bytes }),
  )
}

export function checkNativeLocales(binary, catalogueRoot = catalogue) {
  const expected = readInterfaceLanguages(catalogueRoot)
  const metadata = embeddedPlist(binary)
  assert.equal(metadata.CFBundleDevelopmentRegion, expected.fallback)
  assert.deepEqual(metadata.CFBundleLocalizations, expected.languages)
  return expected
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  assert.equal(process.platform, 'darwin', 'Mach-O locale verification requires macOS')
  assert.ok(process.argv[2], 'usage: node app/scripts/check-native-locales.mjs <executable>')
  const expected = checkNativeLocales(resolve(process.argv[2]))
  console.log(
    `Native locales verified: ${expected.languages.join(', ')} (fallback: ${expected.fallback})`,
  )
}
