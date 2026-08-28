// WS9 Phase 9f — Synthetic topology matching the reference architecture

import type {
  AgentNode,
  MeshEdge,
  FlowEvent,
  MeshTopologySnapshot,
  EdgeType,
} from './types';

/** Reference architecture nodes */
const MOCK_AGENTS: AgentNode[] = [
  { id: 'harness',        peerId: 'Qm-harness',    health: 'healthy', currentTasks: 3, skills: ['orchestration', 'llm-dispatch'], techniques: [],          role: 'orchestrator', position: [0, 4, 0] },
  { id: 'nexus-server',   peerId: 'Qm-server',     health: 'healthy', currentTasks: 1, skills: ['grpc', 'a2a'],                  techniques: [],          role: 'server',       position: [0, 0, 0] },
  { id: 'field-agent-1',  peerId: 'Qm-field1',     health: 'healthy', currentTasks: 2, skills: ['recon', 'exec'],                techniques: ['T1059'],   role: 'field-agent',  position: [-6, -2, 3] },
  { id: 'field-agent-2',  peerId: 'Qm-field2',     health: 'degraded', currentTasks: 1, skills: ['exec', 'persistence'],         techniques: ['T1547'],   role: 'field-agent',  position: [-3, -3, -4] },
  { id: 'field-agent-3',  peerId: 'Qm-field3',     health: 'healthy', currentTasks: 0, skills: ['lateral'],                      techniques: ['T1021'],   role: 'field-agent',  position: [5, -2, -2] },
  { id: 'kali',           peerId: 'Qm-kali',       health: 'healthy', currentTasks: 1, skills: ['pentest', 'tools'],             techniques: [],          role: 'toolbox',      position: [7, 1, 4] },
  { id: 'console',        peerId: 'Qm-console',    health: 'healthy', currentTasks: 0, skills: ['ui', 'ops'],                    techniques: [],          role: 'console',      position: [-7, 3, -1] },
  { id: 'rtpi-backend',   peerId: 'Qm-rtpi-be',    health: 'healthy', currentTasks: 2, skills: ['engagement', 'state'],          techniques: [],          role: 'backend',      position: [3, 5, -3] },
  { id: 'rtpi-frontend',  peerId: 'Qm-rtpi-fe',    health: 'healthy', currentTasks: 0, skills: ['ui'],                           techniques: [],          role: 'frontend',     position: [5, 6, -1] },
  { id: 'mesh-relay',     peerId: 'Qm-relay',      health: 'healthy', currentTasks: 0, skills: ['relay', 'dtn'],                 techniques: [],          role: 'relay',        position: [0, -5, 0] },
  { id: 'swarm-node',     peerId: 'Qm-swarm',      health: 'healthy', currentTasks: 1, skills: ['consensus', 'vote'],            techniques: [],          role: 'swarm',        position: [-4, -5, 3] },
];

/** 23 edges spanning the reference architecture */
const MOCK_EDGES: MeshEdge[] = [
  // Ferry links (harness <-> server <-> agents)
  { source: 'harness',       target: 'nexus-server',  edgeType: 'ferry',     trafficRate: 120, isActive: true },
  { source: 'nexus-server',  target: 'field-agent-1', edgeType: 'ferry',     trafficRate: 80,  isActive: true },
  { source: 'nexus-server',  target: 'field-agent-2', edgeType: 'ferry',     trafficRate: 45,  isActive: true },
  { source: 'nexus-server',  target: 'field-agent-3', edgeType: 'ferry',     trafficRate: 60,  isActive: true },
  // libp2p mesh
  { source: 'field-agent-1', target: 'mesh-relay',    edgeType: 'libp2p',    trafficRate: 30,  isActive: true },
  { source: 'field-agent-2', target: 'mesh-relay',    edgeType: 'libp2p',    trafficRate: 25,  isActive: true },
  { source: 'field-agent-3', target: 'mesh-relay',    edgeType: 'libp2p',    trafficRate: 20,  isActive: true },
  { source: 'mesh-relay',    target: 'nexus-server',  edgeType: 'libp2p',    trafficRate: 90,  isActive: true },
  { source: 'mesh-relay',    target: 'swarm-node',    edgeType: 'libp2p',    trafficRate: 15,  isActive: true },
  // Telemetry
  { source: 'field-agent-1', target: 'nexus-server',  edgeType: 'telemetry', trafficRate: 10,  isActive: true },
  { source: 'field-agent-2', target: 'nexus-server',  edgeType: 'telemetry', trafficRate: 8,   isActive: true },
  { source: 'field-agent-3', target: 'nexus-server',  edgeType: 'telemetry', trafficRate: 12,  isActive: true },
  { source: 'kali',          target: 'nexus-server',  edgeType: 'telemetry', trafficRate: 5,   isActive: true },
  { source: 'mesh-relay',    target: 'nexus-server',  edgeType: 'telemetry', trafficRate: 7,   isActive: false },
  // Swarm
  { source: 'swarm-node',    target: 'field-agent-1', edgeType: 'swarm',     trafficRate: 4,   isActive: true },
  { source: 'swarm-node',    target: 'field-agent-2', edgeType: 'swarm',     trafficRate: 3,   isActive: true },
  { source: 'swarm-node',    target: 'field-agent-3', edgeType: 'swarm',     trafficRate: 4,   isActive: true },
  // Kernel
  { source: 'harness',       target: 'rtpi-backend',  edgeType: 'kernel',    trafficRate: 50,  isActive: true },
  { source: 'rtpi-backend',  target: 'rtpi-frontend', edgeType: 'kernel',    trafficRate: 40,  isActive: true },
  { source: 'console',       target: 'nexus-server',  edgeType: 'kernel',    trafficRate: 35,  isActive: true },
  { source: 'console',       target: 'harness',       edgeType: 'kernel',    trafficRate: 20,  isActive: true },
  { source: 'kali',          target: 'field-agent-1', edgeType: 'ferry',     trafficRate: 15,  isActive: false },
  { source: 'nexus-server',  target: 'kali',          edgeType: 'ferry',     trafficRate: 25,  isActive: true },
];

const EDGE_TYPES: EdgeType[] = ['ferry', 'libp2p', 'telemetry', 'swarm', 'kernel'];
let seqNum = 0;

function randomFlow(agents: AgentNode[], edges: MeshEdge[]): FlowEvent {
  const edge = edges[Math.floor(Math.random() * edges.length)];
  return {
    source: edge.source,
    target: edge.target,
    edgeType: edge.edgeType,
    taskId: `task-${Math.random().toString(36).slice(2, 8)}`,
    payloadBytes: Math.floor(Math.random() * 4096) + 64,
    timestamp: Date.now(),
  };
}

/** Generate a full snapshot with jittered health and random flows */
export function generateMockSnapshot(): MeshTopologySnapshot {
  // Deep-clone agents so positions can be mutated by layout
  const agents = MOCK_AGENTS.map((a) => ({
    ...a,
    position: [...a.position] as [number, number, number],
  }));

  // Random health jitter on one agent
  if (Math.random() < 0.15) {
    const idx = Math.floor(Math.random() * agents.length);
    agents[idx].health = Math.random() < 0.5 ? 'degraded' : 'unresponsive';
  }

  // 2-5 random flow events
  const flowCount = 2 + Math.floor(Math.random() * 4);
  const flows: FlowEvent[] = [];
  for (let i = 0; i < flowCount; i++) {
    flows.push(randomFlow(agents, MOCK_EDGES));
  }

  const globalScore = 0.15 + Math.random() * 0.5;
  const perAgent = new Map<string, number>();
  for (const a of agents) {
    perAgent.set(a.id, Math.random() * 0.6);
  }

  return {
    agents,
    edges: [...MOCK_EDGES],
    flows,
    barometer: {
      globalScore,
      perAgent,
      controllerState: globalScore < 0.3 ? 'nominal' : globalScore < 0.7 ? 'throttling' : 'kill-switch',
      throttleFloor: 0.1,
    },
    sequenceNumber: seqNum++,
    timestamp: Date.now(),
  };
}
