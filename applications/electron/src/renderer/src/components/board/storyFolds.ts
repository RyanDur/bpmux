type Keyed = { id: number }

export const upsertById = <T extends Keyed>(items: T[], incoming: T): T[] => {
  const index = items.findIndex((item) => item.id === incoming.id)
  if (index === -1) return [...items, incoming]
  return items.map((item, i) => (i === index ? incoming : item))
}

export const removeById = <T extends Keyed>(items: T[], id: number): T[] =>
  items.filter((item) => item.id !== id)

export const orderByIds = <T extends Keyed>(items: T[], ids: number[]): T[] => {
  const byId = new Map(items.map((item) => [item.id, item]))
  const ordered = ids.flatMap((id) => {
    const found = byId.get(id)
    if (found === undefined) return []
    byId.delete(id)
    return [found]
  })
  return [...ordered, ...items.filter((item) => byId.has(item.id))]
}
