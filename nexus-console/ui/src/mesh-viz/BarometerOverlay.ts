// WS9 Phase 9d — HTML overlay for GML noise barometer + ambient light control

import * as THREE from 'three';
import { BAROMETER_COLORS } from './constants';

export class BarometerOverlay {
  element: HTMLDivElement;
  private ambientLight: THREE.AmbientLight;
  private scoreSpan: HTMLSpanElement;
  private stateSpan: HTMLSpanElement;
  private bar: HTMLDivElement;

  constructor(container: HTMLElement, ambientLight: THREE.AmbientLight) {
    this.ambientLight = ambientLight;

    this.element = document.createElement('div');
    Object.assign(this.element.style, {
      position: 'absolute',
      top: '12px',
      right: '12px',
      background: 'rgba(10, 14, 23, 0.85)',
      border: '1px solid rgba(255,255,255,0.15)',
      borderRadius: '8px',
      padding: '10px 14px',
      color: '#e2e8f0',
      fontFamily: 'monospace',
      fontSize: '12px',
      zIndex: '10',
      minWidth: '160px',
    });

    this.element.innerHTML = `
      <div style="font-weight:600;margin-bottom:6px;">Noise Barometer</div>
      <div style="display:flex;align-items:center;gap:8px;margin-bottom:4px;">
        <span>Score:</span>
        <span id="baro-score" style="font-weight:600;">0.00</span>
      </div>
      <div id="baro-bar" style="height:4px;border-radius:2px;background:#1e293b;overflow:hidden;">
        <div style="height:100%;width:0%;transition:width 0.3s;border-radius:2px;"></div>
      </div>
      <div style="margin-top:6px;">
        <span>State: </span>
        <span id="baro-state">nominal</span>
      </div>
    `;

    container.appendChild(this.element);

    this.scoreSpan = this.element.querySelector('#baro-score') as HTMLSpanElement;
    this.stateSpan = this.element.querySelector('#baro-state') as HTMLSpanElement;
    this.bar = this.element.querySelector('#baro-bar > div') as HTMLDivElement;
  }

  /** Update the overlay and ambient light from barometer state */
  update(globalScore: number, controllerState: string): void {
    const clamped = Math.max(0, Math.min(1, globalScore));

    // Text
    this.scoreSpan.textContent = clamped.toFixed(2);
    this.stateSpan.textContent = controllerState;

    // Bar fill
    this.bar.style.width = `${clamped * 100}%`;

    // Color interpolation: blue -> amber -> red
    let hex: number;
    if (clamped < 0.3) {
      hex = BAROMETER_COLORS.low;
      this.bar.style.background = '#3b82f6';
    } else if (clamped < 0.7) {
      hex = BAROMETER_COLORS.mid;
      this.bar.style.background = '#f59e0b';
    } else {
      hex = BAROMETER_COLORS.high;
      this.bar.style.background = '#ef4444';
    }

    // Shift ambient light to match
    this.ambientLight.color.setHex(hex);
    this.ambientLight.intensity = 0.3 + clamped * 0.4;

    // Score text color follows bar
    this.scoreSpan.style.color = this.bar.style.background;
  }

  dispose(): void {
    this.element.remove();
  }
}
