import * as THREE from 'three';

/**
 * SunSystem - Manages a moving sun with dynamic lighting
 *
 * Creates a realistic sun that moves across the sky, providing
 * natural day/night cycle lighting with proper shadows and colors.
 */
export class SunSystem {
  private sun: THREE.Mesh;
  private sunLight: THREE.DirectionalLight;
  private ambientLight: THREE.AmbientLight;
  private hemisphereLight: THREE.HemisphereLight;
  private scene: THREE.Scene;
  private timeOfDay: number = 0.3; // 0 = midnight, 0.5 = noon, 1.0 = midnight again
  private sunSpeed: number = 0.02; // Speed of sun movement (lower = slower)
  private autoMove: boolean = true;
  private sunDistance: number = 200; // Distance from center

  constructor(scene: THREE.Scene) {
    this.scene = scene;

    // Create visual sun
    const sunGeometry = new THREE.SphereGeometry(8, 32, 32);
    const sunMaterial = new THREE.MeshBasicMaterial({
      color: 0xffffee,
    });
    this.sun = new THREE.Mesh(sunGeometry, sunMaterial);
    this.sun.layers.enable(1); // Enable for bloom layer
    scene.add(this.sun);

    // Create directional light (sun light)
    this.sunLight = new THREE.DirectionalLight(0xffffee, 1.5);
    this.sunLight.castShadow = true;

    // Improved shadow quality
    this.sunLight.shadow.mapSize.width = 2048;
    this.sunLight.shadow.mapSize.height = 2048;
    this.sunLight.shadow.camera.near = 0.5;
    this.sunLight.shadow.camera.far = 500;
    this.sunLight.shadow.camera.left = -100;
    this.sunLight.shadow.camera.right = 100;
    this.sunLight.shadow.camera.top = 100;
    this.sunLight.shadow.camera.bottom = -100;
    this.sunLight.shadow.bias = -0.0005;
    this.sunLight.shadow.normalBias = 0.05;

    scene.add(this.sunLight);
    scene.add(this.sunLight.target);

    // Create ambient light
    this.ambientLight = new THREE.AmbientLight(0xffffff, 0.4);
    scene.add(this.ambientLight);

    // Create hemisphere light for sky/ground ambient
    this.hemisphereLight = new THREE.HemisphereLight(0x87ceeb, 0x4a5f3a, 0.6);
    scene.add(this.hemisphereLight);

    // Initial position
    this.updateSunPosition();
  }

  /**
   * Update sun position and lighting based on time of day
   */
  private updateSunPosition(): void {
    // Calculate sun position in a circular path
    // Time 0 = midnight (sun below horizon)
    // Time 0.25 = sunrise
    // Time 0.5 = noon (sun at peak)
    // Time 0.75 = sunset
    const angle = this.timeOfDay * Math.PI * 2;

    // Sun moves in an arc across the sky
    // Offset angle by -PI/2 so that timeOfDay=0.5 (noon) gives maximum Y
    const sunX = Math.sin(angle) * this.sunDistance;
    const sunY = -Math.cos(angle) * this.sunDistance; // Negated so noon is at top
    const sunZ = Math.cos(angle * 0.5) * 50; // Slight Z variation for visual interest

    this.sun.position.set(sunX, sunY, sunZ);
    this.sunLight.position.copy(this.sun.position);
    this.sunLight.target.position.set(0, 0, 0);

    // Update lighting based on sun height (Y position)
    const normalizedHeight = (sunY + this.sunDistance) / (this.sunDistance * 2); // 0 to 1

    // Day/night color transitions
    if (normalizedHeight > 0.5) {
      // Daytime
      const dayProgress = (normalizedHeight - 0.5) * 2; // 0 to 1 for day
      const sunColor = new THREE.Color().lerpColors(
        new THREE.Color(0xffaa66), // Dawn/dusk orange
        new THREE.Color(0xffffee), // Bright noon
        dayProgress
      );
      this.sunLight.color = sunColor;
      if (this.sun.material instanceof THREE.MeshBasicMaterial) {
        this.sun.material.color = sunColor;
      }
      this.sunLight.intensity = 1.2 + dayProgress * 0.5; // 1.2 to 1.7

      // Day ambient
      this.ambientLight.intensity = 0.4 + dayProgress * 0.2; // 0.4 to 0.6
      this.hemisphereLight.intensity = 0.6 + dayProgress * 0.2; // 0.6 to 0.8
      this.hemisphereLight.color = new THREE.Color(0x87ceeb); // Sky blue
      this.hemisphereLight.groundColor = new THREE.Color(0x4a5f3a); // Ground green
    } else {
      // Dawn/dusk/night
      const nightProgress = normalizedHeight * 2; // 0 to 1 for night (0 = deep night, 1 = dawn)
      const sunColor = new THREE.Color().lerpColors(
        new THREE.Color(0x4444aa), // Deep night blue
        new THREE.Color(0xff8844), // Dawn/dusk orange
        nightProgress
      );
      this.sunLight.color = sunColor;
      if (this.sun.material instanceof THREE.MeshBasicMaterial) {
        this.sun.material.color = sunColor;
      }
      this.sunLight.intensity = 0.2 + nightProgress * 1.0; // 0.2 to 1.2

      // Night ambient
      this.ambientLight.intensity = 0.1 + nightProgress * 0.3; // 0.1 to 0.4
      this.hemisphereLight.intensity = 0.2 + nightProgress * 0.4; // 0.2 to 0.6
      this.hemisphereLight.color = new THREE.Color().lerpColors(
        new THREE.Color(0x1a1a3a), // Night sky
        new THREE.Color(0xff8844), // Dawn sky
        nightProgress
      );
      this.hemisphereLight.groundColor = new THREE.Color(0x0a0f1a); // Dark ground
    }

    // Make sun visible only when above horizon
    this.sun.visible = normalizedHeight > 0.1;
  }

  /**
   * Update sun animation
   * @param deltaTime Time elapsed since last frame in seconds
   */
  update(deltaTime: number): void {
    if (this.autoMove) {
      this.timeOfDay += deltaTime * this.sunSpeed;
      if (this.timeOfDay > 1.0) {
        this.timeOfDay -= 1.0;
      }
      this.updateSunPosition();
    }
  }

  /**
   * Set time of day (0 to 1, where 0.5 is noon)
   */
  setTimeOfDay(time: number): void {
    this.timeOfDay = Math.max(0, Math.min(1, time));
    this.updateSunPosition();
  }

  /**
   * Get current time of day
   */
  getTimeOfDay(): number {
    return this.timeOfDay;
  }

  /**
   * Set sun movement speed
   */
  setSunSpeed(speed: number): void {
    this.sunSpeed = speed;
  }

  /**
   * Toggle automatic sun movement
   */
  setAutoMove(auto: boolean): void {
    this.autoMove = auto;
  }

  /**
   * Get sun light for external access
   */
  getSunLight(): THREE.DirectionalLight {
    return this.sunLight;
  }

  /**
   * Clean up resources
   */
  dispose(): void {
    this.scene.remove(this.sun);
    this.scene.remove(this.sunLight);
    this.scene.remove(this.ambientLight);
    this.scene.remove(this.hemisphereLight);
    this.sun.geometry.dispose();
    if (this.sun.material instanceof THREE.Material) {
      this.sun.material.dispose();
    }
  }
}
