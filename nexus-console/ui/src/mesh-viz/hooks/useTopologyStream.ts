// WS9 Phase 9d — React hook: topology stream from Tauri events or mock data

import { useEffect, useRef } from 'react';
import { useTopologyStore } from '../TopologyStore';
import { generateMockSnapshot } from '../MockTopologyProvider';
import type { MeshTopologySnapshot } from '../types';

/** Detect Tauri runtime */
function isTauri(): boolean {
  return typeof (window as any).__TAURI_INTERNALS__ !== 'undefined';
}

/**
 * Connects to the topology data source:
 *  - Tauri mode: listens to `topology-update` events via @tauri-apps/api/event
 *  - Mock mode: generates synthetic snapshots on an interval
 */
export function useTopologyStream(mockIntervalMs = 1500): void {
  const setSnapshot = useTopologyStore((s) => s.setSnapshot);
  const isMockMode = useTopologyStore((s) => s.isMockMode);
  const setMockMode = useTopologyStore((s) => s.setMockMode);
  const cleanupRef = useRef<(() => void) | null>(null);

  useEffect(() => {
    let cancelled = false;

    if (isTauri() && !isMockMode) {
      // Tauri event listener — dynamic import so non-Tauri envs don't break
      import('@tauri-apps/api/event')
        .then(({ listen }) => {
          if (cancelled) return;
          listen<MeshTopologySnapshot>('topology-update', (event) => {
            setSnapshot(event.payload);
          }).then((unlisten) => {
            cleanupRef.current = unlisten;
          });
        })
        .catch(() => {
          // Tauri API unavailable — fall back to mock
          if (!cancelled) setMockMode(true);
        });
    } else {
      // Mock mode — generate snapshots on interval
      setMockMode(true);

      // Initial snapshot
      setSnapshot(generateMockSnapshot());

      const timer = setInterval(() => {
        if (!cancelled) {
          setSnapshot(generateMockSnapshot());
        }
      }, mockIntervalMs);

      cleanupRef.current = () => clearInterval(timer);
    }

    return () => {
      cancelled = true;
      cleanupRef.current?.();
      cleanupRef.current = null;
    };
  }, [isMockMode, mockIntervalMs, setSnapshot, setMockMode]);
}
