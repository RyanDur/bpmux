import { describe, expect, test } from 'vitest'
import { validateConnectForm } from '../githubConnectForm'

const filled = {
  owner: 'acme',
  repo: 'bpmux',
  token: 'ghp_secret',
  enterprise: false,
  baseUrl: ''
}

describe('every visible field is required before a connect leaves', () => {
  test('a complete hosted form has no errors', () => {
    expect(validateConnectForm(filled)).toEqual({})
  })

  test('a complete enterprise form has no errors', () => {
    expect(
      validateConnectForm({ ...filled, enterprise: true, baseUrl: 'https://ghe.example.com/' })
    ).toEqual({})
  })

  test('blank and whitespace-only fields each name what to enter', () => {
    expect(validateConnectForm({ ...filled, owner: ' ', repo: '', token: '\t' })).toEqual({
      owner: 'enter the owner',
      repo: 'enter the repository',
      token: 'enter a token'
    })
  })

  test('enterprise demands the API base URL; hosted never mentions it', () => {
    expect(validateConnectForm({ ...filled, enterprise: true })).toEqual({
      baseUrl: 'enter the API base URL'
    })
    expect(validateConnectForm({ ...filled, baseUrl: '' })).toEqual({})
  })
})
