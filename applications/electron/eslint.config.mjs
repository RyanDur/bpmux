import { defineConfig } from 'eslint/config'
import tseslint from '@electron-toolkit/eslint-config-ts'
import eslintConfigPrettier from '@electron-toolkit/eslint-config-prettier'
import eslintPluginReact from 'eslint-plugin-react'
import eslintPluginReactHooks from 'eslint-plugin-react-hooks'
import eslintPluginReactRefresh from 'eslint-plugin-react-refresh'
import eslintPluginTestingLibrary from 'eslint-plugin-testing-library'
import eslintPluginJestDom from 'eslint-plugin-jest-dom'
import eslintPluginJsxA11y from 'eslint-plugin-jsx-a11y'
import eslintPluginPreferArrow from 'eslint-plugin-prefer-arrow-functions'

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
      '**/*.tsbuildinfo'
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
      'prefer-arrow-functions': eslintPluginPreferArrow
    },
    rules: {
      ...eslintPluginReactHooks.configs.recommended.rules,
      ...eslintPluginReactRefresh.configs.vite.rules,
      'func-style': ['error', 'expression', { allowArrowFunctions: true }],
      'prefer-arrow-functions/prefer-arrow-functions': ['error', { returnStyle: 'unchanged' }],
      'prefer-arrow-callback': 'error',
      'prefer-const': 'error',
      '@typescript-eslint/no-explicit-any': 'error',
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
    files: ['src/renderer/src/**/*.tsx'],
    ignores: ['**/*.test.{ts,tsx}', '**/fixtures/**'],
    rules: {
      'no-restricted-syntax': [
        'error',
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
    }
  },
  {
    files: ['**/*.test.{ts,tsx}', '**/__tests__/**/*.{ts,tsx}', 'tests/**/*.{ts,tsx}'],
    plugins: {
      'testing-library': eslintPluginTestingLibrary,
      'jest-dom': eslintPluginJestDom
    },
    rules: {
      'testing-library/prefer-user-event': 'warn',
      'testing-library/no-container': 'warn',
      'testing-library/no-unnecessary-act': 'warn',
      'testing-library/prefer-screen-queries': 'warn',
      'testing-library/prefer-find-by': 'warn',
      'testing-library/no-debugging-utils': 'warn',
      'jest-dom/prefer-enabled-disabled': 'warn'
    }
  },
  eslintConfigPrettier
)
