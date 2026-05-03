# Interface

**Goal: Make it easy for people to compile, run, and debug code on the web.** There's a lot of complexity inherit in all three of these things–we'll focus on providing a simple interface that's good enough for most users.

## Desiderata

The interface we should provide should:

- Accept code from one of several langauges (e.g. `Python`, `C`. `C++`, `Rust`, etc.), which can grow over time as the library evolves, all of which share a common runtime interface.
- Transparently communicate execution errors to the client (compiler, linker, user errors, etc).
- Indicate the phase of execution of the user code (preparing, compiling, linking, running, done).
- Provide methods for interfacing with standard in, out, and error.
- Allow configuring the initial state of the virtual filesystem.
- Provide a debugging interface which allows:
  - Reading/modifying which breakpoint locations have been set
  - Stepping into, out of, or over the current stop point
  - On break, reading the stack trace with the state of locals for each frame

## Usage

Here's a few examples of how the library would look:

```ts
/**
 * Creates a C runtime.
 *
 * Note that this is an async function as it may need to do some initialization of WASM modules, etc.
 * Question: can this method be made synchronous?? Nicer interface
 */
const runtime = await Runtime.create('c');

/**
 * The `fs` field controls the *initial* filesystem that the code will be invoked on.
 * It will not update dynamically as the program modifies its own filesystem while running.
 */
runtime.fs = { 'main.c': 'int main() { /* ... */ }' };

runtime.stdout.on('data', (chunk) => console.log(new TextDecoder().decode(chunk)));
void runtime.stdin.write('haha ');

/**
 * Compiles, links, and runs the program to completion
 * Awaiting it means we won't go past this line until its finished executing.
 */
await runtime.run();

/**
 * Calling `run` multiple times in a row without awaiting it will do nothing.
 */
runtime.run();
runtime.run();
runtime.run();

/**
 * However, runtimes are re-usable between runs.
 * This causes the code to be run twice:
 */
await runtime.run();
await runtime.run();

/**
 * `stop` kills the on-going execution, enabling it to be ran again.
 */
runtime.run();
runtime.stop();

/**
 * `stop` does nothing if the runtime is already stopped
 */
runtime.stop();

/** Error handling */

const result = await runtime.run();
console.log(result.success); // false
console.log(result.stage); // preparing, compiling, linking, running
console.log(result.error); // Error(...)

/**
 * Debugger interface
 */
runtime.debugger.setBreakpoint('main.c:1');
runtime.debugger.onBreakpoint((bp) => console.log(bp.locals));

runtime.debugger.pause(); // Pause wherever we are currently executing
runtime.debugger.resume(); // Keep going after a breakpoint was hit

runtime.debugger.stepInto(); // Steps into, assuming we are paused
runtime.debugger.stepOut(); // Steps out, assuming we are paused
runtime.debugger.step(); // Steps to next line, assuming we are paused
```
