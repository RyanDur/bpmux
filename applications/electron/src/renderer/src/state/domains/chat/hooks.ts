import { useSelector } from '../../selecting'
import type { ChatMessage } from '@transport/types'

export const useLiveMessages = (): ChatMessage[] => useSelector((s) => s.messages)
