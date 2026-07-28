import { describe, expect, test } from 'vitest'
import { renderToStaticMarkup } from 'react-dom/server'
import GithubSettings from '../GithubSettings'

const baseProps = {
  apiBase: undefined,
  authToken: undefined,
  ownerNonce: undefined,
  onClose: (): void => {}
}

describe('GithubSettings dialog is declarative (no ref)', () => {
  test('open=true reflects the open attribute and names the surface', () => {
    const html = renderToStaticMarkup(<GithubSettings {...baseProps} open={true} />)
    expect(html).toMatch(/<dialog\b[^>]*\bopen\b/)
    expect(html).toContain('aria-label="GitHub settings"')
    expect(html).toContain('aria-label="Close settings"')
  })

  test('open=false omits the open attribute declaratively', () => {
    const html = renderToStaticMarkup(<GithubSettings {...baseProps} open={false} />)
    expect(html).not.toMatch(/<dialog\b[^>]*\bopen\b/)
  })
})

describe('the connect form is complete before any network settles', () => {
  test('owner, repo, and a password-typed token field render with the submit affordance', () => {
    const html = renderToStaticMarkup(<GithubSettings {...baseProps} open={true} />)
    expect(html).toContain('data-test="gh-owner"')
    expect(html).toContain('data-test="gh-repo"')
    expect(html).toMatch(
      /data-test="gh-token"[^>]*type="password"|type="password"[^>]*data-test="gh-token"/
    )
    expect(html).toContain('data-test="gh-connect"')
  })

  test('the live-sync truth is stated on the surface — no restart language survives', () => {
    const html = renderToStaticMarkup(<GithubSettings {...baseProps} open={true} />)
    expect(html).toContain('live · always current')
    expect(html).not.toContain('restart')
  })
})

describe('the enterprise reveal', () => {
  test('the checkbox renders and the API base field stays hidden until checked', async () => {
    const { render, screen } = await import('@testing-library/react')
    const { default: userEvent } = await import('@testing-library/user-event')
    render(<GithubSettings {...baseProps} open={true} />)
    expect(screen.getByText('GitHub Enterprise')).toBeTruthy()
    expect(screen.queryByText('API base URL')).toBeNull()
    await userEvent.click(screen.getByRole('checkbox'))
    expect(screen.getByText('API base URL')).toBeTruthy()
  })
})
