import { useSelector } from '../../selecting'
import type { ConnectionStatus } from '@transport/types'

export const useConnectionStatus = (): ConnectionStatus => useSelector((s) => s.status)
