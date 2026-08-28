// WS9 Phase 9d — Root React component for the 3D mesh visualization

import React, { useEffect, useRef } from 'react';
import { useTopologyStore } from './TopologyStore';
import { useTopologyStream } from './hooks/useTopologyStream';
import { SceneManager } from './SceneManager';
import { NodeRenderer } from './NodeRenderer';
import { EdgeRenderer } from './EdgeRenderer';
import { FlowParticleSystem } from './FlowParticleSystem';
import { BarometerOverlay } from './BarometerOverlay';
import { layoutTick, resetLayout } from './LayoutEngine';

export const MeshVisualization: React.FC = () => {
  const containerRef = useRef<HTMLDivElement>(null);
  const sceneRef = useRef<SceneManager | null>(null);
  const nodesRef = useRef<NodeRenderer | null>(null);
  const edgesRef = useRef<EdgeRenderer | null>(null);
  const particlesRef = useRef<FlowParticleSystem | null>(null);
  const barometerRef = useRef<BarometerOverlay | null>(null);
  const prevSeqRef = useRef<number>(-1);

  // Connect to topology data source (Tauri events or mock)
  useTopologyStream();

  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    // Initialize renderers
    const scene = new SceneManager(container);
    const nodes = new NodeRenderer();
    const edges = new EdgeRenderer();
    const particles = new FlowParticleSystem();
    const barometer = new BarometerOverlay(container, scene.ambientLight);

    scene.scene.add(nodes.mesh);
    scene.scene.add(edges.lineSegments);
    scene.scene.add(particles.mesh);

    sceneRef.current = scene;
    nodesRef.current = nodes;
    edgesRef.current = edges;
    particlesRef.current = particles;
    barometerRef.current = barometer;

    // Animation tick
    scene.setTickCallback((dt) => {
      const snapshot = useTopologyStore.getState().snapshot;
      if (!snapshot) return;

      // Detect new snapshot and spawn flow particles
      if (snapshot.sequenceNumber !== prevSeqRef.current) {
        prevSeqRef.current = snapshot.sequenceNumber;
        particles.spawnFromFlows(snapshot.flows, snapshot.agents);
      }

      // Run force layout
      layoutTick(snapshot.agents, snapshot.edges);

      // Update renderers
      nodes.update(snapshot.agents, snapshot.barometer.perAgent);
      edges.update(snapshot.agents, snapshot.edges);
      particles.update(dt);
      barometer.update(snapshot.barometer.globalScore, snapshot.barometer.controllerState);
    });

    scene.start();

    return () => {
      scene.dispose();
      nodes.dispose();
      edges.dispose();
      particles.dispose();
      barometer.dispose();
      resetLayout();
    };
  }, []);

  return (
    <div
      ref={containerRef}
      style={{
        width: '100%',
        height: '100%',
        position: 'relative',
        minHeight: '400px',
        overflow: 'hidden',
        borderRadius: '8px',
      }}
    />
  );
};
