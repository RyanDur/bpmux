type Props = {
  label: string
  testId: string
  value: string
  onChange: (value: string) => void
  error: string | undefined
  type?: string
  placeholder?: string
}

const ConnectField = ({
  label,
  testId,
  value,
  onChange,
  error,
  type,
  placeholder
}: Props): React.JSX.Element => {
  const errorId = `${testId}-error`
  return (
    <fieldset className="github-settings-field">
      <label>
        {label}
        <input
          data-test={testId}
          type={type ?? 'text'}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          aria-invalid={error !== undefined}
          aria-describedby={error === undefined ? undefined : errorId}
          autoComplete="off"
          placeholder={placeholder}
        />
      </label>
      {error !== undefined && (
        <small id={errorId} className="github-settings-field-error">
          {error}
        </small>
      )}
    </fieldset>
  )
}

export default ConnectField
