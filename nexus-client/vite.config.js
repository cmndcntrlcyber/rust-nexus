import { defineConfig } from 'vite';

export default defineConfig({
  // Vite config for Tauri
  root: 'src',
  clearScreen: false,
  server: {
    port: 3000,
    strictPort: true,
  },
  envPrefix: ['VITE_', 'TAURI_'],
  build: {
    target: ['es2021', 'chrome97', 'safari13'],
    minify: !process.env.TAURI_DEBUG ? 'esbuild' : false,
    sourcemap: !!process.env.TAURI_DEBUG,
    outDir: '../dist',
    emptyOutDir: true,
  },
});
