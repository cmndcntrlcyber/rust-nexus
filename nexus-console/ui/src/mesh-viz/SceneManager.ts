// WS9 Phase 9d — Three.js scene, camera, renderer, controls setup

import * as THREE from 'three';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
import {
  CAMERA_FOV,
  CAMERA_NEAR,
  CAMERA_FAR,
  CAMERA_POSITION,
  CAMERA_LOOK_AT,
  SCENE_BG,
} from './constants';

export class SceneManager {
  scene: THREE.Scene;
  camera: THREE.PerspectiveCamera;
  renderer: THREE.WebGLRenderer;
  controls: OrbitControls;
  ambientLight: THREE.AmbientLight;
  private container: HTMLElement;
  private animationId: number | null = null;
  private onTick: ((dt: number) => void) | null = null;
  private clock: THREE.Clock;

  constructor(container: HTMLElement) {
    this.container = container;
    this.clock = new THREE.Clock();

    // Scene
    this.scene = new THREE.Scene();
    this.scene.background = new THREE.Color(SCENE_BG);

    // Camera
    const aspect = container.clientWidth / Math.max(container.clientHeight, 1);
    this.camera = new THREE.PerspectiveCamera(CAMERA_FOV, aspect, CAMERA_NEAR, CAMERA_FAR);
    this.camera.position.set(...CAMERA_POSITION);
    this.camera.lookAt(...CAMERA_LOOK_AT);

    // Renderer
    this.renderer = new THREE.WebGLRenderer({ antialias: true, alpha: false });
    this.renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    this.renderer.setSize(container.clientWidth, container.clientHeight);
    container.appendChild(this.renderer.domElement);

    // Controls
    this.controls = new OrbitControls(this.camera, this.renderer.domElement);
    this.controls.enableDamping = true;
    this.controls.dampingFactor = 0.08;
    this.controls.minDistance = 5;
    this.controls.maxDistance = 80;

    // Lights
    this.ambientLight = new THREE.AmbientLight(0x3b82f6, 0.4);
    this.scene.add(this.ambientLight);

    const directional = new THREE.DirectionalLight(0xffffff, 0.8);
    directional.position.set(10, 15, 10);
    this.scene.add(directional);

    // Resize observer
    const ro = new ResizeObserver(() => this.handleResize());
    ro.observe(container);
  }

  private handleResize(): void {
    const w = this.container.clientWidth;
    const h = Math.max(this.container.clientHeight, 1);
    this.camera.aspect = w / h;
    this.camera.updateProjectionMatrix();
    this.renderer.setSize(w, h);
  }

  /** Set the per-frame callback */
  setTickCallback(cb: (dt: number) => void): void {
    this.onTick = cb;
  }

  /** Start the render loop */
  start(): void {
    const loop = () => {
      this.animationId = requestAnimationFrame(loop);
      const dt = this.clock.getDelta();
      this.controls.update();
      this.onTick?.(dt);
      this.renderer.render(this.scene, this.camera);
    };
    loop();
  }

  /** Stop the render loop */
  stop(): void {
    if (this.animationId !== null) {
      cancelAnimationFrame(this.animationId);
      this.animationId = null;
    }
  }

  /** Full cleanup */
  dispose(): void {
    this.stop();
    this.controls.dispose();
    this.renderer.dispose();
    this.renderer.domElement.remove();
  }
}
