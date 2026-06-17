/// <reference types="vite/client" />

declare const __CROSSWORLD_VERSION__: string

interface Window {
  nostr?: {
    getPublicKey(): Promise<string>;
    signEvent(event: any): Promise<any>;
  };
}
