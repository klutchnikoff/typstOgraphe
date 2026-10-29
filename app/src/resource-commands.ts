import type { Provider, Transport, Resource } from './provider'
import type { StructuredCommand } from './structured-commands'
import { tr } from './i18n'

export type ImageResource = Resource
export const resourceCommand = (resource: ImageResource): StructuredCommand => ({
  id: 'resource.add',
  params: { resource },
})
export function installResources(provider: Provider, resources: ImageResource[]) {
  for (const resource of resources) provider.addResource(resource)
}
/** Same pure decoder for document imports and global profile images. */
export async function readImage(transport: Transport, file: File) {
  const data = new Uint8Array(await file.arrayBuffer())
  let binary = ''
  for (let start = 0; start < data.length; start += 8192)
    binary += String.fromCharCode(...data.subarray(start, start + 8192))
  return transport.invoke<ImageResource>('prepare_image', {
    extension: file.name.split('.').pop()?.toLowerCase() || '',
    data: btoa(binary),
  })
}
/** Reading/decoding never admits a resource. Only the guarded batch can install it. */
export async function prepareImage(provider: Provider, transport: Transport, file: File) {
  const generation = provider.generation,
    identity = provider.doc.getMap('settings').get('identity')
  const resource = await readImage(transport, file)
  if (
    generation !== provider.generation ||
    identity !== provider.doc.getMap('settings').get('identity')
  )
    throw new Error(tr('Le document a changé. Réessayez.'))
  return resource
}
