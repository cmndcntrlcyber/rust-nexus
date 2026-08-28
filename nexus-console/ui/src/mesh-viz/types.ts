// WS9 Phase 9d — Mesh visualization type definitions

export type HealthStatus = 'healthy' | 'degraded' | 'unresponsive';

export type EdgeType = 'ferry' | 'libp2p' | 'telemetry' | 'swarm' | 'kernel';

export interface AgentNode {
  id: string;
  peerId: string;
  health: HealthStatus;
  currentTasks: number;
  skills: string[];
  techniques: string[];
  role: string;
  position: [number, number, number];
}

export interface MeshEdge {
  source: string;
  target: string;
  edgeType: EdgeType;
  trafficRate: number;
  isActive: boolean;
}

export interface FlowEvent {
  source: string;
  target: string;
  edgeType: EdgeType;
  taskId: string;
  payloadBytes: number;
  timestamp: number;
}

export interface NoiseBarometer {
  globalScore: number;
  perAgent: Map<string, number>;
  controllerState: string;
  throttleFloor: number;
}

export interface MeshTopologySnapshot {
  agents: AgentNode[];
  edges: MeshEdge[];
  flows: FlowEvent[];
  barometer: NoiseBarometer;
  sequenceNumber: number;
  timestamp: number;
}
