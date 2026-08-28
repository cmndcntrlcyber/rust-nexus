// WS9 Phase 9d — 3D force-directed layout (spring-electric model)

import type { AgentNode, MeshEdge } from './types';
import {
  REPULSION_STRENGTH,
  ATTRACTION_STRENGTH,
  DAMPING,
  LAYOUT_ITERATIONS_PER_TICK,
} from './constants';

type Vec3 = [number, number, number];

/** Mutable velocity map keyed by agent id */
const velocities = new Map<string, Vec3>();

function getVelocity(id: string): Vec3 {
  if (!velocities.has(id)) velocities.set(id, [0, 0, 0]);
  return velocities.get(id)!;
}

function add(a: Vec3, b: Vec3): Vec3 {
  return [a[0] + b[0], a[1] + b[1], a[2] + b[2]];
}

function scale(v: Vec3, s: number): Vec3 {
  return [v[0] * s, v[1] * s, v[2] * s];
}

function sub(a: Vec3, b: Vec3): Vec3 {
  return [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
}

function length(v: Vec3): number {
  return Math.sqrt(v[0] * v[0] + v[1] * v[1] + v[2] * v[2]);
}

/**
 * Run one tick of force-directed layout.
 * Mutates the position field of each agent in place and returns the array.
 */
export function layoutTick(agents: AgentNode[], edges: MeshEdge[]): AgentNode[] {
  for (let iter = 0; iter < LAYOUT_ITERATIONS_PER_TICK; iter++) {
    const forces = new Map<string, Vec3>();
    for (const a of agents) forces.set(a.id, [0, 0, 0]);

    // Repulsion between all pairs
    for (let i = 0; i < agents.length; i++) {
      for (let j = i + 1; j < agents.length; j++) {
        const a = agents[i];
        const b = agents[j];
        const delta = sub(a.position, b.position);
        const dist = Math.max(length(delta), 0.1);
        const force = REPULSION_STRENGTH / (dist * dist);
        const dir = scale(delta, force / dist);

        forces.set(a.id, add(forces.get(a.id)!, dir));
        forces.set(b.id, add(forces.get(b.id)!, scale(dir, -1)));
      }
    }

    // Attraction along edges
    const agentMap = new Map(agents.map((a) => [a.id, a]));
    for (const edge of edges) {
      const src = agentMap.get(edge.source);
      const tgt = agentMap.get(edge.target);
      if (!src || !tgt) continue;

      const delta = sub(tgt.position, src.position);
      const dist = length(delta);
      if (dist < 0.01) continue;

      const force = ATTRACTION_STRENGTH * dist;
      const dir = scale(delta, force / dist);

      forces.set(src.id, add(forces.get(src.id)!, dir));
      forces.set(tgt.id, add(forces.get(tgt.id)!, scale(dir, -1)));
    }

    // Apply forces with damping
    for (const agent of agents) {
      const vel = getVelocity(agent.id);
      const f = forces.get(agent.id)!;
      const newVel: Vec3 = [
        (vel[0] + f[0]) * DAMPING,
        (vel[1] + f[1]) * DAMPING,
        (vel[2] + f[2]) * DAMPING,
      ];
      velocities.set(agent.id, newVel);
      agent.position = add(agent.position, scale(newVel, 0.016));
    }
  }

  return agents;
}

/** Reset velocity state (call when topology changes structurally) */
export function resetLayout(): void {
  velocities.clear();
}
