// WS9 Phase 9d — Entry point: mount React mesh visualization into #mesh-viz-root

import React from 'react';
import { createRoot } from 'react-dom/client';
import { MeshVisualization } from './MeshVisualization';

const rootEl = document.getElementById('mesh-viz-root');

if (rootEl) {
  // Clear the Leptos placeholder content before mounting React
  rootEl.innerHTML = '';
  const root = createRoot(rootEl);
  root.render(
    <React.StrictMode>
      <MeshVisualization />
    </React.StrictMode>
  );
}
