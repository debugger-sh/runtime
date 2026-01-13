import fs from 'fs';
import { defineConfig } from 'tsup';

export default defineConfig({
  entry: ['src/ts/index.ts'],
  format: ['esm'],
  dts: true,
  sourcemap: true,
  clean: true,
  loader: { '.wasm': 'file' },
  onSuccess: async () => {
    fs.cpSync('pkg', 'dist/pkg', { recursive: true });
  },
});
