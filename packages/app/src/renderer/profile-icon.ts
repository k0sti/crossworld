import * as logger from '../utils/logger';
import * as THREE from 'three';

/**
 * ProfileIcon - 2D sprite that displays a nostr profile picture above an avatar
 */
export class ProfileIcon {
  private sprite: THREE.Sprite;
  private textureLoader: THREE.TextureLoader;
  private currentPictureUrl: string | null = null;
  private defaultTexture: THREE.Texture;
  private displayName: string = '';

  constructor(size: number = 0.8, displayName: string = '') {
    this.textureLoader = new THREE.TextureLoader();
    this.displayName = displayName;

    // Create default texture with initials if name provided
    this.defaultTexture = this.createDefaultTexture();

    // Create sprite material with default texture
    const material = new THREE.SpriteMaterial({
      map: this.defaultTexture,
      transparent: true,
      depthTest: false, // Always render on top
      depthWrite: false,
    });

    this.sprite = new THREE.Sprite(material);
    this.sprite.scale.set(size, size, 1);
    this.sprite.renderOrder = 1000; // Render after everything else
  }

  /**
   * Get initials from display name (up to 2 characters)
   */
  private getInitials(name: string): string {
    if (!name) return '';

    const words = name.trim().split(/\s+/);
    if (words.length === 1) {
      return words[0].slice(0, 2).toUpperCase();
    }
    return (words[0][0] + words[words.length - 1][0]).toUpperCase();
  }

  /**
   * Generate a color from name (similar to Chakra UI Avatar)
   */
  private getColorFromName(name: string): string {
    if (!name) return 'rgba(100, 100, 100, 0.8)';

    let hash = 0;
    for (let i = 0; i < name.length; i++) {
      hash = name.charCodeAt(i) + ((hash << 5) - hash);
    }

    const hue = Math.abs(hash % 360);
    // Reduced saturation and lightness, added alpha for softer look
    return `hsla(${hue}, 35%, 45%, 0.85)`;
  }

  /**
   * Create a default circle texture with initials or grey circle
   */
  private createDefaultTexture(): THREE.Texture {
    const canvas = document.createElement('canvas');
    canvas.width = 128;
    canvas.height = 128;
    const ctx = canvas.getContext('2d');

    if (ctx) {
      const initials = this.getInitials(this.displayName);
      const bgColor = this.getColorFromName(this.displayName);

      // Draw colored circle background
      ctx.fillStyle = bgColor;
      ctx.beginPath();
      ctx.arc(64, 64, 60, 0, Math.PI * 2);
      ctx.fill();

      // Draw initials if available
      if (initials) {
        ctx.fillStyle = 'rgba(255, 255, 255, 0.95)';
        ctx.font = 'bold 48px sans-serif';
        ctx.textAlign = 'center';
        ctx.textBaseline = 'middle';
        ctx.fillText(initials, 64, 64);
      }

      // Draw softer border
      ctx.strokeStyle = 'rgba(255, 255, 255, 0.5)';
      ctx.lineWidth = 3;
      ctx.beginPath();
      ctx.arc(64, 64, 60, 0, Math.PI * 2);
      ctx.stroke();
    }

    const texture = new THREE.CanvasTexture(canvas);
    texture.needsUpdate = true;
    return texture;
  }

  /**
   * Load and display a profile picture from URL
   */
  async loadPicture(pictureUrl: string): Promise<void> {
    if (this.currentPictureUrl === pictureUrl) {
      return; // Already loaded
    }

    try {
      // Wait for texture to fully load
      const texture = await this.textureLoader.loadAsync(pictureUrl);

      // Verify image is loaded
      if (!texture.image || texture.image.width === 0 || texture.image.height === 0) {
        this.currentPictureUrl = null;
        return;
      }

      // Create circular mask
      const maskedTexture = this.createCircularMaskedTexture(texture);

      // Dispose old texture before replacing
      const oldMap = this.sprite.material.map;
      if (oldMap && oldMap !== this.defaultTexture) {
        oldMap.dispose();
      }

      // Update sprite material
      this.sprite.material.map = maskedTexture;
      this.sprite.material.needsUpdate = true;

      // Mark as successfully loaded
      this.currentPictureUrl = pictureUrl;
    } catch (error) {
      logger.error('renderer', '[ProfileIcon] Failed to load profile picture:', error);
      // Reset on error to keep default texture
      this.currentPictureUrl = null;
    }
  }

  /**
   * Create a circular masked version of the texture with border
   */
  private createCircularMaskedTexture(texture: THREE.Texture): THREE.Texture {
    const canvas = document.createElement('canvas');
    const size = 128;
    canvas.width = size;
    canvas.height = size;
    const ctx = canvas.getContext('2d');

    if (ctx) {
      // Save state before clipping
      ctx.save();

      // Create circular clip path
      ctx.beginPath();
      ctx.arc(size / 2, size / 2, size / 2 - 4, 0, Math.PI * 2);
      ctx.closePath();
      ctx.clip();

      // Draw the image
      const img = texture.image as HTMLImageElement | HTMLCanvasElement;
      if (img && img.width > 0 && img.height > 0) {
        // Calculate scaling to fit circle (cover mode)
        const scale = Math.max(size / img.width, size / img.height);
        const x = (size - img.width * scale) / 2;
        const y = (size - img.height * scale) / 2;
        ctx.drawImage(img, x, y, img.width * scale, img.height * scale);
      }

      // Restore to remove clip and draw border
      ctx.restore();

      // Draw softer border
      ctx.strokeStyle = 'rgba(255, 255, 255, 0.5)';
      ctx.lineWidth = 3;
      ctx.beginPath();
      ctx.arc(size / 2, size / 2, size / 2 - 2, 0, Math.PI * 2);
      ctx.stroke();
    }

    const maskedTexture = new THREE.CanvasTexture(canvas);
    maskedTexture.needsUpdate = true;
    return maskedTexture;
  }

  /**
   * Update display name and regenerate default texture
   */
  setDisplayName(name: string): void {
    if (this.displayName === name) return; // No change needed

    this.displayName = name;

    // Regenerate default texture with new initials
    const oldDefault = this.defaultTexture;
    this.defaultTexture = this.createDefaultTexture();

    // If currently showing default, update to new default
    if (!this.currentPictureUrl) {
      this.sprite.material.map = this.defaultTexture;
      this.sprite.material.needsUpdate = true;
    }

    // Dispose old default texture
    oldDefault.dispose();
  }

  /**
   * Reset to default texture
   */
  resetToDefault(): void {
    this.currentPictureUrl = null;
    if (this.sprite.material.map && this.sprite.material.map !== this.defaultTexture) {
      this.sprite.material.map.dispose();
    }
    this.sprite.material.map = this.defaultTexture;
    this.sprite.material.needsUpdate = true;
  }

  /**
   * Get the sprite object to add to scene
   */
  getSprite(): THREE.Sprite {
    return this.sprite;
  }

  /**
   * Set position relative to avatar (typically above the avatar)
   */
  setPosition(x: number, y: number, z: number): void {
    this.sprite.position.set(x, y, z);
  }

  /**
   * Set visibility
   */
  setVisible(visible: boolean): void {
    this.sprite.visible = visible;
  }

  /**
   * Update size
   */
  setSize(size: number): void {
    this.sprite.scale.set(size, size, 1);
  }

  /**
   * Clean up resources
   */
  dispose(): void {
    if (this.sprite.material.map && this.sprite.material.map !== this.defaultTexture) {
      this.sprite.material.map.dispose();
    }
    this.defaultTexture.dispose();
    this.sprite.material.dispose();
  }
}
