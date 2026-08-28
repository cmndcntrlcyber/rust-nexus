// WS9 Phase 9d — Visual constants for mesh visualization

import type { EdgeType, HealthStatus } from './types';

/** Edge colors by type */
export const EDGE_COLORS: Record<EdgeType, string> = {
  ferry:     '#D85A30', // coral
  libp2p:    '#22d3ee', // cyan
  telemetry: '#1D9E75', // teal
  swarm:     '#f59e0b', // amber
  kernel:    '#a78bfa', // purple
};

/** Node colors by health status */
export const NODE_COLORS: Record<HealthStatus, number> = {
  healthy:      0x22d3ee, // cyan
  degraded:     0xf59e0b, // amber
  unresponsive: 0xef4444, // red
};

/** Emissive glow intensity for anomaly score 0..1 */
export const ANOMALY_EMISSIVE_MAX = 0.6;

/** Node geometry */
export const NODE_RADIUS = 0.35;
export const NODE_SEGMENTS = 16;
export const MAX_NODES = 64;

/** Flow particle geometry */
export const PARTICLE_RADIUS = 0.08;
export const MAX_PARTICLES = 256;
export const PARTICLE_SPEED = 2.0; // units per second

/** Camera defaults */
export const CAMERA_FOV = 60;
export const CAMERA_NEAR = 0.1;
export const CAMERA_FAR = 500;
export const CAMERA_POSITION: [number, number, number] = [0, 8, 20];
export const CAMERA_LOOK_AT: [number, number, number] = [0, 0, 0];

/** Scene background */
export const SCENE_BG = 0x0a0e17;

/** Barometer ambient light color stops: blue -> amber -> red */
export const BAROMETER_COLORS = {
  low:  0x3b82f6, // blue — score < 0.3
  mid:  0xf59e0b, // amber — score 0.3..0.7
  high: 0xef4444, // red — score > 0.7
};

/** Layout engine constants */
export const REPULSION_STRENGTH = 50;
export const ATTRACTION_STRENGTH = 0.05;
export const DAMPING = 0.85;
export const LAYOUT_ITERATIONS_PER_TICK = 3;
