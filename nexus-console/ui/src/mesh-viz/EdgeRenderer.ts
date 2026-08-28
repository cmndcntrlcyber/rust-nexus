// WS9 Phase 9d — LineSegments renderer for mesh edges

import * as THREE from 'three';
import type { AgentNode, MeshEdge } from './types';
import { EDGE_COLORS } from './constants';

export class EdgeRenderer {
  lineSegments: THREE.LineSegments;
  private geometry: THREE.BufferGeometry;
  private material: THREE.LineBasicMaterial;
  private positionAttr: THREE.Float32BufferAttribute;
  private colorAttr: THREE.Float32BufferAttribute;
  private maxEdges: number;

  constructor(maxEdges = 128) {
    this.maxEdges = maxEdges;
    this.geometry = new THREE.BufferGeometry();

    // Each edge = 2 vertices = 6 floats for position, 6 for color
    this.positionAttr = new THREE.Float32BufferAttribute(
      new Float32Array(maxEdges * 6),
      3
    );
    this.positionAttr.setUsage(THREE.DynamicDrawUsage);
    this.geometry.setAttribute('position', this.positionAttr);

    this.colorAttr = new THREE.Float32BufferAttribute(
      new Float32Array(maxEdges * 6),
      3
    );
    this.colorAttr.setUsage(THREE.DynamicDrawUsage);
    this.geometry.setAttribute('color', this.colorAttr);

    this.material = new THREE.LineBasicMaterial({
      vertexColors: true,
      transparent: true,
      opacity: 0.7,
    });

    this.lineSegments = new THREE.LineSegments(this.geometry, this.material);
  }

  /** Rebuild edge geometry from current topology */
  update(agents: AgentNode[], edges: MeshEdge[]): void {
    const agentMap = new Map(agents.map((a) => [a.id, a]));
    const _color = new THREE.Color();
    let idx = 0;

    for (const edge of edges) {
      if (idx >= this.maxEdges) break;
      const src = agentMap.get(edge.source);
      const tgt = agentMap.get(edge.target);
      if (!src || !tgt) continue;

      const base = idx * 6;
      this.positionAttr.array[base]     = src.position[0];
      this.positionAttr.array[base + 1] = src.position[1];
      this.positionAttr.array[base + 2] = src.position[2];
      this.positionAttr.array[base + 3] = tgt.position[0];
      this.positionAttr.array[base + 4] = tgt.position[1];
      this.positionAttr.array[base + 5] = tgt.position[2];

      _color.set(EDGE_COLORS[edge.edgeType]);
      const alpha = edge.isActive ? 1.0 : 0.4;
      this.colorAttr.array[base]     = _color.r * alpha;
      this.colorAttr.array[base + 1] = _color.g * alpha;
      this.colorAttr.array[base + 2] = _color.b * alpha;
      this.colorAttr.array[base + 3] = _color.r * alpha;
      this.colorAttr.array[base + 4] = _color.g * alpha;
      this.colorAttr.array[base + 5] = _color.b * alpha;

      idx++;
    }

    this.geometry.setDrawRange(0, idx * 2);
    this.positionAttr.needsUpdate = true;
    this.colorAttr.needsUpdate = true;
  }

  dispose(): void {
    this.geometry.dispose();
    this.material.dispose();
  }
}
