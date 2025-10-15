// Ready Player Me integration for avatar fetching
// Docs: https://docs.readyplayer.me/

export interface ReadyPlayerMeConfig {
  avatarUrl?: string;
  subdomain?: string;
  bodyType?: 'fullbody' | 'halfbody';
  quality?: 'low' | 'medium' | 'high';
}

export class ReadyPlayerMeService {
  private static readonly DEFAULT_SUBDOMAIN = 'demo';
  private static readonly API_BASE = 'https://models.readyplayer.me';

  /**
   * Get GLB URL from Ready Player Me avatar ID or full URL
   */
  static getAvatarUrl(avatarIdOrUrl: string, config: ReadyPlayerMeConfig = {}): string {
    // If it's already a full URL, use it directly
    if (avatarIdOrUrl.startsWith('http')) {
      return this.addUrlParameters(avatarIdOrUrl, config);
    }

    // Otherwise construct URL from ID
    const baseUrl = `${this.API_BASE}/${avatarIdOrUrl}.glb`;
    return this.addUrlParameters(baseUrl, config);
  }

  /**
   * Add query parameters to customize the avatar
   */
  private static addUrlParameters(url: string, config: ReadyPlayerMeConfig): string {
    const params = new URLSearchParams();

    if (config.quality) {
      params.set('quality', config.quality);
    }

    if (config.bodyType === 'halfbody') {
      params.set('meshLod', '1'); // Half body
    }

    const paramString = params.toString();
    return paramString ? `${url}?${paramString}` : url;
  }

  /**
   * Open Ready Player Me avatar creator
   */
  static openAvatarCreator(subdomain: string = this.DEFAULT_SUBDOMAIN): void {
    const creatorUrl = `https://${subdomain}.readyplayer.me/avatar`;
    window.open(creatorUrl, '_blank');
  }

  /**
   * Get a test/demo avatar URL for debugging
   */
  static getTestAvatarUrl(): string {
    // This is a publicly available test avatar from Ready Player Me
    return 'https://models.readyplayer.me/68efc931e831796787cfe117.glb';
  }

  /**
   * Validate if a URL is a Ready Player Me avatar URL
   */
  static isReadyPlayerMeUrl(url: string): boolean {
    return url.includes('readyplayer.me') || url.includes('models.readyplayer.me');
  }

  /**
   * Extract avatar ID from Ready Player Me URL
   */
  static extractAvatarId(url: string): string | null {
    const match = url.match(/([a-f0-9]{24})\.glb/i);
    return match ? match[1] : null;
  }
}
