import { defineConfig } from 'eslint/config'
import tseslint from '@electron-toolkit/eslint-config-ts'
import eslintPluginReact from 'eslint-plugin-react'
import eslintPluginReactHooks from 'eslint-plugin-react-hooks'
import eslintPluginReactRefresh from 'eslint-plugin-react-refresh'
import eslintPluginTestingLibrary from 'eslint-plugin-testing-library'
import eslintPluginJestDom from 'eslint-plugin-jest-dom'
import eslintPluginJsxA11y from 'eslint-plugin-jsx-a11y'
import eslintPluginPreferArrow from 'eslint-plugin-prefer-arrow-functions'
import eslintPluginSonarjs from 'eslint-plugin-sonarjs'

// Shared no-restricted-syntax selectors. no-restricted-syntax REPLACES (does not
// merge) across flat-config blocks, so a per-file waiver that needs to drop ONE
// selector must re-specify the others. The useRef keep-waivers below re-use
// NO_DIV_SPAN (div/span stay enforced) while omitting NO_USE_REF (ref allowed).
const NO_DIV_SPAN = [
  {
    selector: "JSXOpeningElement[name.name='div']",
    message:
      'no <div> — html5-semantic canon (feedback_html5_semantic_no_divs_spans): use <section>/<article>/<aside>/<main>/<header>/<footer>/<nav>/<form>/<details>/<dialog>/<figure> per element-role; if the role is decorative wrapping for layout, hoist the layout onto the parent via flex/grid gap. waivers: none today; document any genuine library-edge here with file:line + rationale before disabling.'
  },
  {
    selector: "JSXOpeningElement[name.name='span']",
    message:
      'no <span> — html5-semantic canon (feedback_first_rule_of_aria + feedback_html5_semantic_no_divs_spans): use <small>/<b>/<i>/<output>/<data>/<mark>/<em>/<strong>/<time>/<kbd>/<code> per phrasing-role, or CSS ::before/::after for decoration. if you need an inline grouping with no semantic, ask whether the styling can move to the parent. waivers: none today; document any genuine library-edge here with file:line + rationale before disabling.'
  }
]

const NO_USE_REF = {
  selector: "CallExpression[callee.name='useRef']",
  message:
    'no useRef — Ryan-canon "no refs unless no other way" (feedback_no_refs_unless_no_other_way): prefer declarative state/derivation. A ref is correct ONLY when there is no better-AND-safe alternative: DOM-imperative with no declarative API (dialog.showModal/.focus/.scroll/measure), a value read only in a handler/effect that must NOT drive render (useState would be a stale-closure or a wasted/storm render), or an external-system handle/timer. If a ref is genuinely irreducible, add the file to a keep-waiver block below with the ref name + "no other way" reason. KNOWN LIMITATION: waivers are per-FILE — a NEW ref added to an already-waived file is review-caught, not lint-caught (decidable creep into clean files is lint-caught; in-waived-file judgment is review-caught).'
}

export default defineConfig(
  {
    ignores: [
      '**/node_modules/**',
      '**/dist/**',
      '**/out/**',
      '**/build/**',
      '**/test-results/**',
      '**/playwright-report/**',
      '**/coverage/**',
      '**/*.tsbuildinfo',
      '**/test-support/fixtures/**'
    ]
  },
  tseslint.configs.recommended,
  eslintPluginReact.configs.flat.recommended,
  eslintPluginReact.configs.flat['jsx-runtime'],
  eslintPluginJsxA11y.flatConfigs.recommended,
  {
    settings: {
      react: {
        version: 'detect'
      }
    }
  },
  {
    files: ['**/*.{ts,tsx}'],
    plugins: {
      'react-hooks': eslintPluginReactHooks,
      'react-refresh': eslintPluginReactRefresh,
      'prefer-arrow-functions': eslintPluginPreferArrow,
      sonarjs: eslintPluginSonarjs
    },
    rules: {
      ...eslintPluginReactHooks.configs.recommended.rules,
      ...eslintPluginReactRefresh.configs.vite.rules,
      'func-style': ['error', 'expression', { allowArrowFunctions: true }],
      'prefer-arrow-functions/prefer-arrow-functions': ['error', { returnStyle: 'unchanged' }],
      'prefer-arrow-callback': 'error',
      'prefer-const': 'error',
      'sonarjs/cognitive-complexity': ['error', 15],
      'max-depth': ['error', 3],
      'max-params': ['error', 4],
      'max-nested-callbacks': ['error', 3],
      'arrow-body-style': ['error', 'as-needed'],
      'object-shorthand': ['error', 'always'],
      'no-lonely-if': 'error',
      'no-else-return': ['error', { allowElseIf: false }],
      'no-useless-rename': 'error',
      'no-extra-boolean-cast': 'error',
      '@typescript-eslint/no-explicit-any': 'error',
      '@typescript-eslint/consistent-type-definitions': ['error', 'type'],
      '@typescript-eslint/no-unused-vars': [
        'error',
        { argsIgnorePattern: '^_', varsIgnorePattern: '^_' }
      ]
    }
  },
  {
    files: ['**/ErrorBoundary.tsx'],
    rules: {
      'func-style': 'off'
    }
  },
  {
    files: ['src/renderer/src/**/*.{ts,tsx}'],
    ignores: ['**/*.test.{ts,tsx}', '**/fixtures/**'],
    rules: {
      'no-restricted-syntax': ['error', ...NO_DIV_SPAN, NO_USE_REF]
    }
  },
  // useRef keep-waivers: useRef is omitted from no-restricted-syntax for these files
  // (div/span stay enforced via NO_DIV_SPAN). Per-file + tight globs. Three honest
  // states: UNCONDITIONAL (no other way EVER — ref-to-rendered-DOM-node) and
  // CONDITIONAL-until:#N (ref-to-data/resource with an out-of-component home; the
  // cited follow-up's job is to remove the ref + this waiver).
  //
  // UNCONDITIONAL — ref-to-a-rendered-DOM-node (component-intrinsic; no module/service
  // can hold the element) OR own-scroll-position / component-local UI tracker:
  {
    files: [
      'src/renderer/src/components/chat/Reactions.tsx', // pickerRef/addButtonRef/buttonRef — DOM .contains + getBoundingClientRect (click-outside + popover anchor); no declarative API
      'src/renderer/src/components/chat/Input.tsx', // textareaRef — DOM .focus + .setSelectionRange (caret); no declarative API
      'src/renderer/src/components/chat/TerminalView.tsx', // containerRef — xterm.open() mounts the terminal into a DOM element; no declarative API
      'src/renderer/src/components/board/drill-in/DrillIn.tsx', // articleRef — DOM .focus on mount; no declarative API
      'src/renderer/src/components/board/drill-in/DrillInActions.tsx', // textareaRef .focus + rejectDialogRef .showModal/.close (modal — Ryan's exemplar); no declarative API
      'src/renderer/src/AppShell.tsx', // confirmDialogRef — dialog.showModal() for the confirm-before-close modal (focus-trap + Esc-cancel + top-layer); no declarative API
      'src/renderer/src/components/board/drill-in/Timeline.tsx', // detailsRef — DOM .open + .scrollIntoView; no declarative API
      'src/renderer/src/components/board/useBoardNavigation.ts', // asideRef — DOM .querySelector focus mgmt (board keyboard-nav owns the region ref); no declarative API
      'src/renderer/src/components/board/drill-in/Comments.tsx', // commentsListRef/commentTextareaRef — DOM scroll-measure/focus; wasAtBottomRef — reads scroll-position off the component's own element (state = stale-closure/render-storm)
      'src/renderer/src/components/chat/useScrollGutter.ts' // listRef — DOM scroll-measure; atBottomRef — own-scroll-position read in handler; lastLengthRef — component-local list-grew detector (no service home)
    ],
    rules: { 'no-restricted-syntax': ['error', ...NO_DIV_SPAN] }
  },
  // jsx-a11y/no-autofocus waiver — TerminalList terminal-rename focus lifecycle only. Two
  // autoFocus uses, both gated on an explicit user edit-action (F2 / double-click): the
  // rename <input> autoFocuses on OPEN (focus lands in the field), and the terminal
  // <button> autoFocuses on its re-mount on SUBMIT (focus RETURNS to the item, so a
  // keyboard user is not orphaned on <body>). The rule guards page-LOAD autofocus-steal;
  // focus FOLLOWING an explicit edit-action is the accessible behaviour, so the rule is a
  // false-positive here. This is the obvious NO-REF solution for the whole lifecycle — no
  // ref, no no-refs exception spent (Ryan-canon: never spend the no-refs exception when an
  // obvious different solution exists; waive the lesser misfiring lint instead).
  {
    files: ['src/renderer/src/components/sidebar/TerminalList.tsx'],
    rules: { 'jsx-a11y/no-autofocus': 'off' }
  },
  {
    files: [
      '**/*.test.{ts,tsx}',
      '**/*.spec.{ts,tsx}',
      '**/test-support/**/*.{ts,tsx}',
      'test/**/*.{ts,tsx}',
      'tests/**/*.{ts,tsx}'
    ],
    rules: {
      'max-nested-callbacks': ['error', 6]
    }
  },
  {
    files: ['**/*.test.{ts,tsx}', '**/test-support/**/*.{ts,tsx}'],
    plugins: {
      'testing-library': eslintPluginTestingLibrary,
      'jest-dom': eslintPluginJestDom
    },
    rules: {
      'testing-library/prefer-user-event': 'error',
      'testing-library/no-container': 'error',
      'testing-library/no-unnecessary-act': 'error',
      'testing-library/prefer-screen-queries': 'error',
      'testing-library/prefer-find-by': 'error',
      'testing-library/no-debugging-utils': 'error',
      'jest-dom/prefer-enabled-disabled': 'error'
    }
  },
  {
    files: ['**/*.d.ts', 'tests/playwright/notifications.spec.ts'],
    rules: {
      '@typescript-eslint/consistent-type-definitions': 'off'
    }
  },
  // Exhaustiveness canon: a switch WITHOUT a default over a union/enum must arm
  // every case — growth in the union breaks the build here until the site decides.
  // A switch WITH a default has declared itself partial (the reducer idiom:
  // default returns state), and the default is honored as that declaration.
  {
    files: ['src/renderer/src/**/*.{ts,tsx}'],
    ignores: ['**/*.test.{ts,tsx}', '**/__tests__/**', '**/__html5-stub__*'],
    languageOptions: {
      parserOptions: { projectService: true, tsconfigRootDir: import.meta.dirname }
    },
    rules: {
      '@typescript-eslint/switch-exhaustiveness-check': [
        'error',
        { considerDefaultExhaustiveForUnions: true }
      ],
      '@typescript-eslint/no-unnecessary-boolean-literal-compare': 'error'
    }
  }
)
