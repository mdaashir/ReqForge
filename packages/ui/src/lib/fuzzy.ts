/**
 * Simple fuzzy-match scoring for command palette queries.
 *
 * Returns a numeric score (higher = better match) or null if the haystack
 * does not contain all the query characters in order.
 *
 * Bonus matches for consecutive characters and word-boundary hits.
 */
export function fuzzyScore(query: string, haystack: string): number | null {
  if (!query) return 1
  const q = query.toLowerCase()
  const h = haystack.toLowerCase()

  let qi = 0
  let score = 0
  let prevMatched = false

  for (let hi = 0; hi < h.length && qi < q.length; hi++) {
    if (h[hi] === q[qi]) {
      // Consecutive matches get a bonus
      score += prevMatched ? 3 : 1
      // Word-boundary matches get an additional bonus
      if (hi === 0 || /[\s\-_/.]/.test(h[hi - 1] || '')) {
        score += 2
      }
      prevMatched = true
      qi++
    } else {
      prevMatched = false
    }
  }

  return qi === q.length ? score : null
}

/**
 * Rank a list of items by fuzzy match against a label extractor.
 *
 * Items that do not match are excluded. Items with the same score are
 * ordered by their original index.
 */
export function fuzzyRank<T>(
  items: T[],
  query: string,
  getHaystack: (item: T) => string
): T[] {
  if (!query.trim()) return items.slice(0, 50)

  const ranked = items
    .map((item, idx) => ({
      item,
      idx,
      score: fuzzyScore(query, getHaystack(item)),
    }))
    .filter((r): r is { item: T; idx: number; score: number } => r.score !== null)
    .sort((a, b) => b.score - a.score || a.idx - b.idx)

  return ranked.slice(0, 50).map((r) => r.item)
}
