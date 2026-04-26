## Developing and Building

- `npm run build` to build the project.
  - This command builds the Rust components to WASM and then bundles everything into the npm library.
  - It is optimized for quick build times. To build for release, use `npm run build:release`.
- `npm run tools:dap` to run a suite of integration tests against the Debugger Adapter Protocol (DAP).
- `npm run tools:dap -- {test}` to run a specific integration test.

## Contribution Standards

- Keep all contributions as simple and elegant as possible.
- Extra code is not acceptable; prefer the smallest clear solution that solves the problem.
