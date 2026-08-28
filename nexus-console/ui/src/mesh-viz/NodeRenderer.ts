// WS9 Phase 9d — InstancedMesh renderer for agent nodes

import * as THREE from 'three';
import type { AgentNode } from './types';
import {
  NODE_COLORS,
  NODE_RADIUS,
  NODE_SEGMENTS,
  MAX_NODES,
  ANOMALY_EMISSIVE_MAX,
} from './constants';

const _dummy = new THREE.Object3D();
const _color = new THREE.Color();

export class NodeRenderer {
  mesh: THREE.InstancedMesh;
  private material: THREE.MeshStandardMaterial;

  constructor() {
    const geometry = new THREE.SphereGeometry(NODE_RADIUS, NODE_SEGMENTS, NODE_SEGMENTS);
    this.material = new THREE.MeshStandardMaterial({
      metalness: 0.3,
      roughness: 0.6,
    });
    this.mesh = new THREE.InstancedMesh(geometry, this.material, MAX_NODES);
    this.mesh.count = 0;
    this.mesh.instanceMatrix.setUsage(THREE.DynamicDrawUsage);
    this.mesh.instanceColor = new THREE.InstancedBufferAttribute(
      new Float32Array(MAX_NODES * 3),
      3
    );
    this.mesh.instanceColor.setUsage(THREE.DynamicDrawUsage);
  }

  /** Update instance transforms and colors from current agent list */
  update(agents: AgentNode[], anomalyScores: Map<string, number>): void {
    const count = Math.min(agents.length, MAX_NODES);
    this.mesh.count = count;

    for (let i = 0; i < count; i++) {
      const agent = agents[i];
      const [x, y, z] = agent.position;

      _dummy.position.set(x, y, z);
      _dummy.updateMatrix();
      this.mesh.setMatrixAt(i, _dummy.matrix);

      // Base color from health
      const baseHex = NODE_COLORS[agent.health];
      _color.setHex(baseHex);

      // Emissive boost from anomaly score
      const score = anomalyScores.get(agent.id) ?? 0;
      const emissive = score * ANOMALY_EMISSIVE_MAX;
      this.material.emissiveIntensity = emissive;
      this.material.emissive.setHex(0xff4444);

      this.mesh.setColorAt(i, _color);
    }

    this.mesh.instanceMatrix.needsUpdate = true;
    if (this.mesh.instanceColor) {
      this.mesh.instanceColor.needsUpdate = true;
    }
  }

  dispose(): void {
    this.mesh.geometry.dispose();
    this.material.dispose();
  }
}
