// Nostr integration — NIP-07 login, profiles, live events

import {
  generateSecretKey,
  getPublicKey,
  nip19,
  SimplePool,
  type Event,
} from 'nostr-tools';

declare global {
  interface Window {
    nostr?: {
      getPublicKey(): Promise<string>;
      signEvent(event: any): Promise<any>;
    };
  }
}

// --- Configuration ---

const DEFAULT_RELAYS = [
  'wss://strfry.atlantislabs.space/',
  'wss://relay.damus.io',
  'wss://nos.lol',
];

const WORLD_RELAYS = ['wss://strfry.atlantislabs.space/'];

const LIVE_EVENT_D_TAG = 'crossworld-dev';

// --- Types ---

export interface NostrUser {
  pubkey: string;
  npub: string;
  displayName: string;
  avatarUrl?: string;
  isGuest: boolean;
}

// --- Module state ---

let currentUser: NostrUser | null = null;

const pool = new SimplePool();
const chatCallbacks: Array<(msg: { pubkey: string; content: string }) => void> = [];
let chatSub: { close(): void } | null = null;

// --- Login ---

export async function loginWithExtension(): Promise<NostrUser> {
  if (!window.nostr) {
    throw new Error('No Nostr extension found. Install a NIP-07 extension (e.g. nos2x, Alby).');
  }

  const pubkey = await window.nostr.getPublicKey();
  const npub = nip19.npubEncode(pubkey);

  const profile = await fetchProfile(pubkey);

  currentUser = {
    pubkey,
    npub,
    displayName: profile.displayName,
    avatarUrl: profile.avatarUrl,
    isGuest: false,
  };

  return currentUser;
}

export function loginAsGuest(): NostrUser {
  const pubkey = getPublicKey(generateSecretKey());
  const npub = nip19.npubEncode(pubkey);

  currentUser = {
    pubkey,
    npub,
    displayName: `Guest-${pubkey.slice(0, 8)}`,
    isGuest: true,
  };

  return currentUser;
}

export function getUser(): NostrUser | null {
  return currentUser;
}

export function logout(): void {
  currentUser = null;
  if (chatSub) {
    chatSub.close();
    chatSub = null;
  }
  chatCallbacks.length = 0;
}

// --- Profiles ---

export async function fetchProfile(
  pubkey: string,
): Promise<{ displayName: string; avatarUrl?: string }> {
  const events = await pool.querySync(DEFAULT_RELAYS, {
    kinds: [0],
    authors: [pubkey],
    limit: 1,
  });

  if (events.length === 0) {
    return { displayName: pubkey.slice(0, 12) };
  }

  try {
    const meta = JSON.parse(events[0].content);
    return {
      displayName: meta.display_name || meta.name || pubkey.slice(0, 12),
      avatarUrl: meta.picture || undefined,
    };
  } catch {
    return { displayName: pubkey.slice(0, 12) };
  }
}

// --- Live events (NIP-53) ---

export async function joinLiveEvent(): Promise<void> {
  // Find the live event first
  const liveEvents = await pool.querySync(WORLD_RELAYS, {
    kinds: [30311],
    '#d': [LIVE_EVENT_D_TAG],
    limit: 1,
  });

  if (liveEvents.length === 0) {
    console.warn('No live event found with d-tag:', LIVE_EVENT_D_TAG);
  }

  // Build the "a" tag reference: kind:pubkey:d-tag
  const aTagRef = liveEvents.length > 0
    ? `30311:${liveEvents[0].pubkey}:${LIVE_EVENT_D_TAG}`
    : undefined;

  // Subscribe to kind 1311 chat messages for this live event
  const filter: Record<string, any> = {
    kinds: [1311],
    since: Math.floor(Date.now() / 1000),
  };

  if (aTagRef) {
    filter['#a'] = [aTagRef];
  }

  if (chatSub) {
    chatSub.close();
  }

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  chatSub = pool.subscribeMany(WORLD_RELAYS, [filter] as any, {
    onevent(event: Event) {
      const msg = { pubkey: event.pubkey, content: event.content };
      for (const cb of chatCallbacks) {
        cb(msg);
      }
    },
  });
}

export function onChatMessage(
  callback: (msg: { pubkey: string; content: string }) => void,
): void {
  chatCallbacks.push(callback);
}
