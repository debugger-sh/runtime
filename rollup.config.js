import typescript from '@rollup/plugin-typescript';
import { wasm } from '@rollup/plugin-wasm';

export default {
  input: 'src/ts/index.ts',
  output: {
    dir: 'dist',
    format: 'esm',
  },
  plugins: [typescript(), wasm()],
};
