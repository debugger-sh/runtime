import init, * as wasm from '../../pkg/runtime.js';
import wasmBinary from '../../pkg/runtime_bg.wasm';

await init(wasmBinary);
wasm.main();
