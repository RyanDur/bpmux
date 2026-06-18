import type { BoardStory } from './types'

export const storyNavigateLabel = (story: BoardStory): string =>
  `Open story #${story.number} ${story.title}`
