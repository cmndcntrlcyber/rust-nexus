// WS9 Phase 9d — Animated particles flowing along edges

import * as THREE from 'three';
import type { AgentNode, FlowEvent } from './types';
import { EDGE_COLORS, PARTICLE_RADIUS, MAX_PARTICLES, PARTICLE_SPEED } from './constants';

interface ActiveParticle {
  sourcePos: [number, number, number];
  targetPos: [number, number, number];
  progress: number; // 0..1
  color: THREE.Color;
  alive: boolean;
}

const _dummy = new THREE.Object3D();

export class FlowParticleSystem {
  mesh: THREE.InstancedMesh;
  private material: THREE.MeshBasicMaterial;
  private particles: ActiveParticle[] = [];

  constructor() {
    const geometry = new THREE.SphereGeometry(PARTICLE_RADIUS, 6, 6);
    this.material = new THREE.MeshBasicMaterial({
      transparent: true,
      opacity: 0.9,
    });
    this.mesh = new THREE.InstancedMesh(geometry, this.material, MAX_PARTICLES);
    this.mesh.count = 0;
    this.mesh.instanceMatrix.setUsage(THREE.DynamicDrawUsage);
    this.mesh.instanceColor = new THREE.InstancedBufferAttribute(
      new Float32Array(MAX_PARTICLES * 3),
      3
    );
    this.mesh.instanceColor.setUsage(THREE.DynamicDrawUsage);
  }

  /** Spawn particles for new flow events */
  spawnFromFlows(flows: FlowEvent[], agents: AgentNode[]): void {
    const agentMap = new Map(agents.map((a) => [a.id, a]));

    for (const flow of flows) {
      if (this.particles.length >= MAX_PARTICLES) break;
      const src = agentMap.get(flow.source);
      const tgt = agentMap.get(flow.target);
      if (!src || !tgt) continue;

      this.particles.push({
        sourcePos: [...src.position],
        targetPos: [...tgt.position],
        progress: 0,
        color: new THREE.Color(EDGE_COLORS[flow.edgeType]),
        alive: true,
      });
    }
  }

  /** Advance particles and update instance transforms */
  update(dt: number): void {
    // Advance and cull
    for (const p of this.particles) {
      p.progress += dt * PARTICLE_SPEED * 0.3;
      if (p.progress >= 1.0) p.alive = false;
    }
    this.particles = this.particles.filter((p) => p.alive);

    const count = Math.min(this.particles.length, MAX_PARTICLES);
    this.mesh.count = count;

    for (let i = 0; i < count; i++) {
      const p = this.particles[i];
      const t = p.progress;
      const x = p.sourcePos[0] + (p.targetPos[0] - p.sourcePos[0]) * t;
      const y = p.sourcePos[1] + (p.targetPos[1] - p.sourcePos[1]) * t;
      const z = p.sourcePos[2] + (p.targetPos[2] - p.sourcePos[2]) * t;

      _dummy.position.set(x, y, z);
      _dummy.updateMatrix();
      this.mesh.setMatrixAt(i, _dummy.matrix);
      this.mesh.setColorAt(i, p.color);
    }

    if (count > 0) {
      this.mesh.instanceMatrix.needsUpdate = true;
      if (this.mesh.instanceColor) this.mesh.instanceColor.needsUpdate = true;
    }
  }

  dispose(): void {
    this.mesh.geometry.dispose();
    this.material.dispose();
  }
}
