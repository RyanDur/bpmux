import type { Story } from './types'

export const storyNavigateLabel = (story: Story): string => `Open story #${story.id} ${story.title}`
