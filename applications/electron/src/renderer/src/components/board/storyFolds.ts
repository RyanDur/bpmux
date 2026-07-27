import type { Story } from './types'

export const upsertStory = (stories: Story[], incoming: Story): Story[] => {
  const index = stories.findIndex((s) => s.id === incoming.id)
  if (index === -1) return [...stories, incoming]
  return stories.map((s, i) => (i === index ? incoming : s))
}

export const removeStory = (stories: Story[], id: number): Story[] =>
  stories.filter((s) => s.id !== id)

export const orderStories = (stories: Story[], ids: number[]): Story[] => {
  const byId = new Map(stories.map((s) => [s.id, s]))
  const ordered = ids.flatMap((id) => {
    const found = byId.get(id)
    if (found === undefined) return []
    byId.delete(id)
    return [found]
  })
  return [...ordered, ...stories.filter((s) => byId.has(s.id))]
}
