import { SimplePool, type Event } from 'nostr-tools'
import { LIVE_CHAT_D_TAG, APP_PUBKEY, DEFAULT_RELAYS } from '../config'

export interface LiveEventData {
  title: string
  summary: string
  status: string
  streaming_url?: string
  relay_urls: string[]
  image?: string
}

/**
 * Fetch the live event (kind:30311) for the Crossworld instance
 */
export async function fetchLiveEvent(): Promise<LiveEventData | null> {
  const pool = new SimplePool()

  try {
    const event = await pool.get(DEFAULT_RELAYS, {
      kinds: [30311],
      authors: [APP_PUBKEY],
      '#d': [LIVE_CHAT_D_TAG],
    })

    if (!event) {
      console.warn(`Live event not found for d-tag: ${LIVE_CHAT_D_TAG}`)
      return null
    }

    return parseLiveEvent(event)
  } finally {
    pool.close(DEFAULT_RELAYS)
  }
}

/**
 * Subscribe to live event updates
 */
export function subscribeLiveEvent(
  onUpdate: (data: LiveEventData) => void
): () => void {
  const pool = new SimplePool()

  const sub = pool.subscribeMany(
    DEFAULT_RELAYS,
    {
      kinds: [30311],
      authors: [APP_PUBKEY],
      '#d': [LIVE_CHAT_D_TAG],
    },
    {
      onevent(event) {
        const data = parseLiveEvent(event)
        if (data) {
          onUpdate(data)
        }
      },
      oneose() {
        console.log('Live event subscription established')
      },
    }
  )

  return () => {
    sub.close()
    pool.close(DEFAULT_RELAYS)
  }
}

function parseLiveEvent(event: Event): LiveEventData | null {
  const tags = event.tags

  const getTag = (name: string): string | undefined => {
    const tag = tags.find((t) => t[0] === name)
    return tag?.[1]
  }

  const getAllTags = (name: string): string[] => {
    return tags.filter((t) => t[0] === name).map((t) => t[1])
  }

  const title = getTag('title') || 'Crossworld'
  const summary = getTag('summary') || ''
  const status = getTag('status') || 'live'
  const streaming_url = getTag('streaming')
  const relay_urls = getAllTags('relay')
  const image = getTag('image')

  return {
    title,
    summary,
    status,
    streaming_url,
    relay_urls,
    image,
  }
}
