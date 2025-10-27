import * as THREE from 'three';
import { EffectComposer } from 'three/examples/jsm/postprocessing/EffectComposer.js';
import { RenderPass } from 'three/examples/jsm/postprocessing/RenderPass.js';
import { UnrealBloomPass } from 'three/examples/jsm/postprocessing/UnrealBloomPass.js';
import { OutputPass } from 'three/examples/jsm/postprocessing/OutputPass.js';

/**
 * PostProcessing - Manages post-processing effects for the scene
 *
 * Adds bloom, glow, and other visual enhancements to make the scene
 * look more sparkly and natural.
 */
export class PostProcessing {
  private composer: EffectComposer;
  private bloomPass: UnrealBloomPass;
  private renderScene: RenderPass;
  private finalPass: OutputPass;

  constructor(
    renderer: THREE.WebGLRenderer,
    scene: THREE.Scene,
    camera: THREE.Camera
  ) {
    // Create effect composer
    this.composer = new EffectComposer(renderer);
    this.composer.setSize(window.innerWidth, window.innerHeight);

    // Add render pass
    this.renderScene = new RenderPass(scene, camera);
    this.composer.addPass(this.renderScene);

    // Add bloom pass for sparkly, glowing effects
    this.bloomPass = new UnrealBloomPass(
      new THREE.Vector2(window.innerWidth, window.innerHeight),
      1.2,  // strength - how much bloom
      0.6,  // radius - bloom spread
      0.85  // threshold - only bright objects bloom
    );
    this.composer.addPass(this.bloomPass);

    // Add output pass for proper color output
    this.finalPass = new OutputPass();
    this.composer.addPass(this.finalPass);
  }

  /**
   * Render the scene with post-processing
   */
  render(deltaTime?: number): void {
    this.composer.render(deltaTime);
  }

  /**
   * Update post-processing on window resize
   */
  setSize(width: number, height: number): void {
    this.composer.setSize(width, height);
  }

  /**
   * Update bloom parameters
   */
  setBloomParams(strength: number, radius: number, threshold: number): void {
    this.bloomPass.strength = strength;
    this.bloomPass.radius = radius;
    this.bloomPass.threshold = threshold;
  }

  /**
   * Get the effect composer
   */
  getComposer(): EffectComposer {
    return this.composer;
  }

  /**
   * Dispose of resources
   */
  dispose(): void {
    this.composer.dispose();
  }
}
