export type ConnectForm = {
  owner: string
  repo: string
  token: string
  enterprise: boolean
  baseUrl: string
}

export type FieldName = 'owner' | 'repo' | 'token' | 'baseUrl'

export type FieldErrors = Partial<Record<FieldName, string>>

const blank = (value: string): boolean => value.trim() === ''

export const validateConnectForm = (form: ConnectForm): FieldErrors => ({
  ...(blank(form.owner) ? { owner: 'enter the owner' } : {}),
  ...(blank(form.repo) ? { repo: 'enter the repository' } : {}),
  ...(blank(form.token) ? { token: 'enter a token' } : {}),
  ...(form.enterprise && blank(form.baseUrl) ? { baseUrl: 'enter the API base URL' } : {})
})

export const hasErrors = (errors: FieldErrors): boolean =>
  Object.values(errors).some((message) => message !== undefined)
