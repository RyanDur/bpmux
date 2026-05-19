import { render, screen, fireEvent, waitFor, act } from '@testing-library/react'
import { expect } from 'vitest'

export const useFireEventAntiPattern = (): void => {
  const { getByRole } = render(<button>Click</button>)
  fireEvent.click(getByRole('button'))
}

export const useContainerAntiPattern = (): void => {
  const { container } = render(<div className="btn" />)
  container.querySelector('.btn')
}

export const useUnnecessaryActAntiPattern = (): void => {
  act(() => {
    fireEvent.click(screen.getByRole('button'))
  })
}

export const useWaitForGetByAntiPattern = async (): Promise<void> => {
  await waitFor(() => screen.getByRole('button'))
}

export const useScreenDebugAntiPattern = (): void => {
  screen.debug()
}

export const useJestDomToBeDisabledAntiPattern = (): void => {
  const button = screen.getByRole('button') as HTMLButtonElement
  expect(button.disabled).toBe(true)
}
