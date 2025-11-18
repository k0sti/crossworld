// Derive APP_PUBKEY from APP_NPUB
import { nip19 } from 'nostr-tools'

export const DEFAULT_RELAYS = [
  'wss://strfry.atlantislabs.space/',
  'wss://relay.damus.io',
  'wss://nos.lol',
  // 'wss://relay.primal.net', // Disabled: CORS issues on localhost (enable for production)
]

export const DEFAULT_RELAY_STATES = {
  'wss://strfry.atlantislabs.space/': { enabledForProfile: false, enabledForWorld: true },
  'wss://relay.damus.io': { enabledForProfile: true, enabledForWorld: false },
  'wss://nos.lol': { enabledForProfile: true, enabledForWorld: false },
  // 'wss://relay.primal.net': { enabledForProfile: true, enabledForWorld: false },
}

// World relays for client status and chat
export const WORLD_RELAYS = ['wss://strfry.atlantislabs.space/']

// Crossworld app identity
export const APP_NPUB = 'npub1ga6mzn7ygwuxpytr264uw09huwef9ypzfda767088gv83ypgtjtsxf25vh'
export const APP_PUBKEY = nip19.decode(APP_NPUB).data as string
