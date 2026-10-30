import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import path from 'node:path'
import { withMcp } from './mcp-client.mjs'
import { bodyTarget, lastBody, catalogueShape, until, hash } from './closure-tools.mjs'

export async function attachedRecipe(binary, invitation, cli, project, catalogue) {
  return withMcp(binary, ['--mcp', '--invitation', invitation], async ({ request, call }) => {
    const tools = await request('tools/list')
    assert.ok(
      !tools.tools.some((t) => /save|export|accept/.test(t.name)),
      'Attached file or acceptance permission leaked',
    )
    assert.deepEqual(
      catalogueShape((await call('commands.list')).outcome),
      catalogueShape(catalogue),
    )
    let inspected = await call('document.inspect')
    assert.deepEqual(inspected.outcome.state, cli(['inspect', project]).document.state)
    const locked = cli(
      [
        'run',
        'metadata.set',
        project,
        '--params',
        JSON.stringify({ target: { kind: 'option', name: 'paper' }, value: 'a5' }),
        '--save',
      ],
      false,
    )
    assert.equal(locked.code, 'unavailable')
    assert.equal(locked.params.reason, 'file_locked')
    const denied = await call(
      'proposals.submit',
      {
        document: inspected.document,
        request_id: 'closure-header-denied',
        summary: 'Not presented by attached review',
        steps: [
          {
            id: 'metadata.set',
            params: { target: { kind: 'option', name: 'paper' }, value: 'a5' },
          },
        ],
      },
      false,
    )
    assert.equal(denied.ok, false)
    const statuses = []
    for (const [id, status, marker] of [
      ['closure-refused', 'rejected', '[MCP-REFUSED]'],
      ['closure-stale', 'stale', '[MCP-STALE]'],
      ['closure-accepted', 'accepted', '[MCP-ACCEPTED]'],
    ]) {
      inspected = await call('document.inspect')
      const structure = await call('document.structure', { document: inspected.document })
      const zone = lastBody(structure.outcome)
      const steps = [
        {
          id: 'text.insert',
          params: { target: bodyTarget(zone), text: marker, marks: [{ type: 'bold' }] },
        },
      ]
      if (status === 'accepted')
        steps.push({
          id: 'inline.replace',
          params: {
            target: bodyTarget({ ...zone, length: zone.length + marker.length }),
            node: { type: 'footnote', attrs: { text: 'Note de l’agent A.2' } },
          },
        })
      const submitted = await call('proposals.submit', {
        document: structure.document,
        request_id: id,
        summary: `Recette A.2 — ${id}`,
        steps,
      })
      assert.equal(submitted.outcome.status, 'pending')
      // Human input may advance the document immediately after submission. Only
      // the agent marker requires approval; do not reject concurrent human work.
      const published = JSON.stringify((await call('document.inspect')).outcome.state).includes(
        marker,
      )
      if (published)
        assert.equal(
          (await call('proposals.get', { proposal: submitted.outcome.id })).outcome.status,
          'accepted',
          'Proposal published without approval',
        )
      await until(
        async () =>
          (await call('proposals.get', { proposal: submitted.outcome.id })).outcome.status ===
          status,
        id,
      )
      statuses.push(status)
    }
    return {
      status: 'passed',
      tools: tools.tools.length,
      statuses,
      lockedWriter: locked.code,
      headerProposalRefused: true,
    }
  })
}

export async function autonomousRecipe(binary, root, project, expected, catalogue, cli) {
  return withMcp(
    binary,
    ['mcp', '--read-root', root, '--write-root', root],
    async ({ request, call }) => {
      assert.deepEqual(
        catalogueShape((await call('commands.list')).outcome),
        catalogueShape(catalogue),
      )
      let response = await call('document.open', { path: project })
      let reference = response.document
      const inspect = async () => (await call('document.inspect')).outcome
      assert.deepEqual((await inspect()).state, expected)
      const before = await inspect(),
        sourceFile = await readFile(project)
      const structure = (await call('document.structure', { document: reference })).outcome
      const zone = lastBody(structure)
      const malformed = await call(
        'document.batch',
        {
          document: reference,
          steps: [
            { id: 'text.insert', params: { target: bodyTarget(zone), text: '[MCP-PRIVATE]' } },
            {
              id: 'text.delete',
              params: { target: { zone: { kind: 'body', path: [999] }, from: 0, to: 1 } },
            },
          ],
          save: { kind: 'current' },
        },
        false,
      )
      assert.equal(malformed.error.code, 'batch_failed')
      assert.deepEqual(await inspect(), before, 'Failed MCP batch changed state/context/history')
      assert.ok((await readFile(project)).equals(sourceFile), 'Failed MCP batch changed archive')
      response = await call('context.set', {
        document: reference,
        selection: {
          kind: 'text',
          anchor: { zone: zone.zone, offset: zone.length },
          head: { zone: zone.zone, offset: zone.length },
        },
        typing_styles: [{ type: 'italic' }],
      })
      assert.deepEqual(response.document.guard, reference.guard)
      assert.deepEqual((await inspect()).state, expected, 'Private context changed the document')
      const stale = reference
      response = await call('context.insert', { document: reference, text: '[MCP-AUTONOMOUS]' })
      reference = response.document
      const tableIndex = (await inspect()).state.document.content.findIndex(
        (n) => n.type === 'tableFigure',
      )
      const textEdited = (await inspect()).state
      if (tableIndex >= 0) {
        response = await call('document.batch', {
          document: reference,
          steps: [
            {
              id: 'table.column.insert',
              params: { target: { root: { kind: 'body' }, path: [tableIndex] }, index: 1 },
            },
          ],
        })
        reference = response.document
        response = await call('edit.undo', { document: reference })
        reference = response.document
        assert.deepEqual(
          (await inspect()).state,
          textEdited,
          'Table undo changed another contribution',
        )
        response = await call('edit.redo', { document: reference })
        reference = response.document
      }
      const edited = (await inspect()).state
      assert.notDeepEqual(edited, expected)
      const outdated = await call('document.text', { document: stale }, false)
      assert.equal(outdated.error.code, 'stale_revision')
      if (tableIndex >= 0) {
        response = await call('edit.undo', { document: reference })
        reference = response.document
      }
      response = await call('edit.undo', { document: reference })
      reference = response.document
      assert.deepEqual((await inspect()).state, expected, 'MCP undo changed another contribution')
      response = await call('edit.redo', { document: reference })
      reference = response.document
      if (tableIndex >= 0) {
        response = await call('edit.redo', { document: reference })
        reference = response.document
      }
      assert.deepEqual((await inspect()).state, edited)
      assert.equal(
        cli(
          [
            'run',
            'text.insert',
            project,
            '--params',
            JSON.stringify({ target: bodyTarget(zone), text: 'locked' }),
            '--save',
          ],
          false,
        ).code,
        'unavailable',
      )
      const forbidden = await call(
        'document.export',
        { document: reference, directory: '../not-authorized', format: 'both' },
        false,
      )
      assert.equal(forbidden.ok, false)
      assert.equal(
        (await call('document.validate', { document: reference })).outcome.ready_for_export,
        true,
      )
      const pages = (await call('render.compile', { document: reference })).outcome.pages
      const png = await request('tools/call', {
        name: 'render.page',
        arguments: { document: reference, page: 1 },
      })
      const image = png.content.find((c) => c.type === 'image')
      assert.ok(
        image &&
          Buffer.from(image.data, 'base64')
            .subarray(0, 8)
            .equals(Buffer.from([137, 80, 78, 71, 13, 10, 26, 10])),
      )
      await call('document.save', { document: reference })
      const directory = path.join(root, 'mcp-export')
      await call('document.export', { document: reference, directory, format: 'both' })
      await call('document.close', { document: reference })
      assert.equal((await call('document.inspect')).outcome.open, false)
      return {
        status: 'passed',
        state: edited,
        pages,
        directory,
        checks: {
          atomicRefusal: true,
          privateContext: true,
          staleGuard: outdated.error.code,
          undoRedo: true,
          tableColumn: tableIndex >= 0,
          independentRoots: true,
          lock: true,
          png: hash(Buffer.from(image.data, 'base64')),
        },
      }
    },
    root,
  )
}
