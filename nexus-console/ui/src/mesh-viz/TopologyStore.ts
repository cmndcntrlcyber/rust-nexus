// WS9 Phase 9d — Zustand store for mesh topology state

import { create } from 'zustand';
import type { MeshTopologySnapshot, AgentNode, MeshEdge, FlowEvent } from './types';

interface TopologyState {
  snapshot: MeshTopologySnapshot | null;
  isMockMode: boolean;

  /** Replace the entire snapshot (from Tauri event or mock provider) */
  setSnapshot: (snap: MeshTopologySnapshot) => void;

  /** Merge incremental flow events into the current snapshot */
  appendFlows: (flows: FlowEvent[]) => void;

  /** Update a single agent's health/position */
  updateAgent: (id: string, patch: Partial<AgentNode>) => void;

  /** Toggle mock mode */
  setMockMode: (mock: boolean) => void;
}

export const useTopologyStore = create<TopologyState>((set) => ({
  snapshot: null,
  isMockMode: false,

  setSnapshot: (snap) => set({ snapshot: snap }),

  appendFlows: (flows) =>
    set((state) => {
      if (!state.snapshot) return state;
      return {
        snapshot: {
          ...state.snapshot,
          flows: [...state.snapshot.flows, ...flows].slice(-200),
        },
      };
    }),

  updateAgent: (id, patch) =>
    set((state) => {
      if (!state.snapshot) return state;
      return {
        snapshot: {
          ...state.snapshot,
          agents: state.snapshot.agents.map((a) =>
            a.id === id ? { ...a, ...patch } : a
          ),
        },
      };
    }),

  setMockMode: (mock) => set({ isMockMode: mock }),
}));
