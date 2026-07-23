module.exports = {
  forbidden: [
    {
      name: 'no-circular',
      comment:
        'A cycle means two modules own each other — nobody owns either. Break it by moving the shared piece to the owner both sides already depend on.',
      severity: 'error',
      from: {},
      to: { circular: true }
    },
    {
      name: 'transport-is-a-door',
      comment:
        'The store is the engine; transport is a door that registers in. A door consuming features inverts the app. Known debt rides in stateSyncDecoders/types (the board type-mirror, fix is codegen) — new reaches are a violation.',
      severity: 'error',
      from: {
        path: '^src/renderer/src/transport',
        pathNot: '(^src/renderer/src/transport/(stateSyncDecoders|types)\\.ts$|\\.test\\.(ts|tsx)$)'
      },
      to: {
        path: '^src/renderer/src/(chat|board|terminal|roles|sidebar|metrics|ci|sync|state)/'
      }
    },
    {
      name: 'the-engine-consumes-no-doors-but-transport',
      comment:
        'state/ may read wire types from transport (the door supplies frames), never from main or preload directly.',
      severity: 'error',
      from: { path: '^src/renderer/src/state/' },
      to: { path: '^src/(main|preload)/' }
    },
    {
      name: 'renderer-stays-in-its-process',
      comment:
        'The electron process boundary: renderer code never imports main or preload modules — they talk over IPC, not imports.',
      severity: 'error',
      from: { path: '^src/renderer/' },
      to: { path: '^src/(main|preload)/' }
    },
    {
      name: 'main-stays-in-its-process',
      severity: 'error',
      from: { path: '^src/(main|preload)/' },
      to: { path: '^src/renderer/' }
    },
    {
      name: 'production-never-imports-tests',
      comment:
        'A prod module importing a test module or test-support smuggles test seams into the product.',
      severity: 'error',
      from: {
        path: '^src/',
        pathNot: '(\\.test\\.(ts|tsx)$|\\.page\\.ts$|test-support/|\\.fixtures\\.ts$)'
      },
      to: { path: '(\\.test\\.(ts|tsx)$|test-support/)' }
    }
  ],
  options: {
    doNotFollow: { path: 'node_modules' },
    exclude: { path: 'test-support/fixtures/dependency-cycle' },
    tsConfig: { fileName: 'tsconfig.web.json' },
    tsPreCompilationDeps: true
  }
}
