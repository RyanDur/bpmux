import AxeBuilder from '@axe-core/playwright'
import type { Page } from '@playwright/test'

type A11yViolationNode = {
  readonly target: readonly string[]
  readonly html: string
  readonly failureSummary: string | null
}

type A11yViolation = {
  readonly id: string
  readonly impact: 'minor' | 'moderate' | 'serious' | 'critical' | null
  readonly description: string
  readonly help: string
  readonly helpUrl: string
  readonly nodes: readonly A11yViolationNode[]
}

export type A11yAuditResult = {
  readonly violations: readonly A11yViolation[]
  readonly passes: number
  readonly incomplete: number
  readonly url: string
}

const AXE_TAGS = ['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa']

export const runA11yAudit = async (page: Page): Promise<A11yAuditResult> => {
  const builder = new AxeBuilder({ page }).withTags(AXE_TAGS).setLegacyMode(true)
  const raw = await builder.analyze()
  return {
    violations: raw.violations.map((v) => ({
      id: v.id,
      impact: (v.impact ?? null) as A11yViolation['impact'],
      description: v.description,
      help: v.help,
      helpUrl: v.helpUrl,
      nodes: v.nodes.map((n) => ({
        target: n.target.map((t) => String(t)),
        html: n.html,
        failureSummary: n.failureSummary ?? null
      }))
    })),
    passes: raw.passes.length,
    incomplete: raw.incomplete.length,
    url: raw.url
  }
}

export const formatViolations = (result: A11yAuditResult): string => {
  if (result.violations.length === 0) {
    return `a11y audit: no violations across ${result.passes} passed checks (${result.incomplete} incomplete)`
  }
  const lines: string[] = [
    `a11y audit: ${result.violations.length} violations (${result.passes} passed, ${result.incomplete} incomplete)`
  ]
  for (const v of result.violations) {
    lines.push(`  [${v.impact ?? 'unknown'}] ${v.id}: ${v.help}`)
    for (const n of v.nodes.slice(0, 3)) {
      lines.push(`    → ${n.target.join(' > ')}`)
      if (n.failureSummary) {
        lines.push(`      ${n.failureSummary.replace(/\n/g, ' ')}`)
      }
    }
    if (v.nodes.length > 3) {
      lines.push(`    … and ${v.nodes.length - 3} more`)
    }
  }
  return lines.join('\n')
}
