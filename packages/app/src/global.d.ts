interface Window {
  nostr?: {
    getPublicKey(): Promise<string>;
    signEvent(event: any): Promise<any>;
  };
}

declare module '@workspace/wasm' {
  export * from '../../../packages/wasm/crossworld_world';
}
