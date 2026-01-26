# Summary: Terminal stdin integration and simplification

## Overview

Integrated interactive stdin input from the xterm.js terminal into the runtime execution system, added control sequences, fixed UI state management, and **simplified the entire I/O architecture by replacing streams with callbacks**.

---

## Part 1: API Simplification

### Problem

The original implementation used Web Streams API which required complex setup:

- `WritableStream` creation for stdout/stderr
- `pipeTo()` with `AbortController` for cleanup
- `stdin.getWriter()` + `TextEncoder` for input
- Manual stream lifecycle management

### Solution

Replaced streams with simpler callback-based API.

#### 1.1 Runtime.ts — new simplified API

Added `pushStdin()` method for direct input:

```typescript
/**
 * Push text to the program's stdin.
 * This is the simplest way to send input to a running program.
 */
public pushStdin(text: string): void {
  const encoded = this.encoder.encode(text);
  this.in.writeSync(encoded);
}
```

Added callback-based `run()` options:

```typescript
export type RunOptions = {
  /** Called when the program writes to stdout. */
  onStdout?: (data: string) => void;
  /** Called when the program writes to stderr. */
  onStderr?: (data: string) => void;
  /** AbortSignal to cancel execution. */
  signal?: AbortSignal;
};

async run(options?: RunOptions): Promise<void> {
  const { onStdout, onStderr, signal } = options ?? {};

  // Handle abort signal
  if (signal?.aborted) return;
  const abortHandler = () => this.stop();
  signal?.addEventListener('abort', abortHandler);

  // Set up callback-based stdout/stderr handling
  const stdoutCallback = onStdout
    ? (event: MessageEvent<WorkerOut>) => {
        if (msg.type !== 'stdout' || msg.mode !== 1) return;
        const text = this.decoder.decode(msg.data as Uint8Array);
        onStdout(text.replaceAll('\n', '\r\n')); // Newline normalization built-in
      }
    : null;
  // ... similar for stderr
}
```

Added `isRunning` getter:

```typescript
public get isRunning(): boolean {
  return this.currentWorker !== null;
}
```

Marked old stream API as deprecated:

```typescript
/** @deprecated Use `pushStdin()` for simpler API */
public get stdin() { return this.in.stream; }

/** @deprecated Use `run({ onStdout })` for simpler API */
public get stdout() { return this.out.stream; }
```

#### 1.2 StdinStream — added synchronous write

```typescript
/**
 * Synchronously write data to the stdin buffer.
 * Used by Runtime.pushStdin() for simpler API.
 */
public writeSync(chunk: Uint8Array): void {
  // Same SharedArrayBuffer + Atomics mechanism, just synchronous
  // ...
}
```

**Key insight**: Streams were just a wrapper around `SharedArrayBuffer` + `Atomics`. The underlying mechanism is unchanged—we simplified the API layer.

---

## Part 2: useCodeExecution hook

### Problem

All execution logic was inline in CodeEditor.tsx (~200 lines), making it hard to maintain.

### Solution

Extracted logic into a reusable hook.

#### 2.1 New hook: `demo/hooks/useCodeExecution.ts`

```typescript
type UseCodeExecutionOptions = {
  terminalRef: React.RefObject<TerminalHandle | null>;
  terminalReady: boolean;
};

type UseCodeExecutionResult = {
  isRunning: boolean;
  runCode: (code: string) => Promise<void>;
  stopCode: () => void;
};

export function useCodeExecution({ terminalRef, terminalReady }: UseCodeExecutionOptions): UseCodeExecutionResult {
  const [isRunning, setIsRunning] = useState(false);
  const runtimeRef = useRef<Runtime | null>(null);
  const isRunningRef = useRef(false);

  // Persistent stdin handler (set up once when terminal is ready)
  useEffect(() => {
    if (!terminalReady) return;
    const terminal = terminalRef.current?.getTerminal();
    if (!terminal) return;

    let stdinBuffer = '';

    const onData = terminal.onData((data: string) => {
      if (!isRunningRef.current || !runtimeRef.current) return;

      // Control sequences
      if (data === '\x03') {
        // Ctrl+C: Stop execution
        terminal.write('^C\r\n');
        runtimeRef.current.stop();
        return;
      } else if (data === '\x04') {
        // Ctrl+D: Send EOF
        terminal.write('^D\r\n');
        runtimeRef.current.pushStdin('\x04');
        stdinBuffer = '';
        return;
      } else if (data === '\x0c') {
        // Ctrl+L: Clear terminal
        terminalRef.current?.clear();
        stdinBuffer = '';
        return;
      }

      // Regular input handling
      if (data === '\r') {
        terminal.write('\r\n');
        runtimeRef.current.pushStdin(`${stdinBuffer}\n`);
        stdinBuffer = '';
      } else if (data === '\u007f') {
        if (stdinBuffer.length > 0) {
          stdinBuffer = stdinBuffer.slice(0, -1);
          terminal.write('\b \b');
        }
      } else if (/* arrow keys */) {
        return;
      } else {
        stdinBuffer += data;
        terminal.write(data);
      }
    });

    return () => onData.dispose();
  }, [terminalRef, terminalReady]);

  const runCode = useCallback(async (code: string) => {
    if (isRunningRef.current) return;

    setIsRunning(true);
    isRunningRef.current = true;
    terminalRef.current?.clear();

    const rt = Runtime.create('c');
    runtimeRef.current = rt;
    rt.fs = { 'main.c': code };
    terminalRef.current?.enableInput();

    try {
      await rt.run({
        onStdout: (text) => terminalRef.current?.write(text),
        onStderr: (text) => terminalRef.current?.write(text),
      });
    } catch (error) {
      terminalRef.current?.writeln(`\r\nError: ${error}`);
    } finally {
      runtimeRef.current = null;
      isRunningRef.current = false;
      setIsRunning(false);
      terminalRef.current?.disableInput();
    }
  }, [terminalRef]);

  const stopCode = useCallback(() => {
    runtimeRef.current?.stop();
  }, []);

  return { isRunning, runCode, stopCode };
}
```

**Key improvements**:

- Stdin handler is persistent (set up once), not recreated each run
- No more manual handler disposal tracking
- No more `AbortController` for pipe cleanup
- No more `WritableStream` creation
- Cleaner separation of concerns

---

## Part 3: Terminal ready callback

### Problem

The `useEffect` in the hook ran on mount, but the terminal wasn't ready yet (async dynamic imports). Stdin handler never got set up.

### Solution

#### 3.1 Terminal.tsx — added onReady callback

```typescript
type TerminalProps = {
  height?: number | string;
  onReady?: () => void; // NEW
};

const Terminal = React.forwardRef<TerminalHandle, TerminalProps>(
  ({ height = 180, onReady }, ref) => {
    // Ref to store callback (avoids re-running effect)
    const onReadyRef = React.useRef(onReady);
    onReadyRef.current = onReady;

    React.useEffect(() => {
      const setupTerminal = async () => {
        // ... dynamic imports and setup ...

        terminalRef.current = term;

        // Notify parent that terminal is ready
        onReadyRef.current?.();
      };
      // ...
    }, []);
  }
);
```

#### 3.2 CodeEditor.tsx — track terminal ready state

```typescript
const [terminalReady, setTerminalReady] = useState(false);

const handleTerminalReady = useCallback(() => {
  setTerminalReady(true);
}, []);

const { isRunning, runCode, stopCode } = useCodeExecution({ terminalRef, terminalReady });

// In JSX:
<Terminal ref={terminalRef} height={terminalHeight} onReady={handleTerminalReady} />
```

---

## Part 4: Simplified CodeEditor

### Before: ~510 lines with inline complexity

- Manual `WritableStream` creation
- `pipeTo()` with `AbortController`
- `stdin.getWriter()` + `TextEncoder`
- Handler disposal tracking with refs
- Stderr timeout heuristics for error detection

### After: ~285 lines using the hook

```typescript
export default function CodeEditor() {
  const [code, setCode] = useState<string>(defaultCode);
  const [terminalReady, setTerminalReady] = useState(false);
  const terminalRef = useRef<TerminalHandle | null>(null);

  const { isRunning, runCode, stopCode } = useCodeExecution({ terminalRef, terminalReady });

  const handleRun = () => runCode(code);

  // ... UI code only, no execution logic ...

  <Button onClick={isRunning ? stopCode : handleRun}>
    {isRunning ? 'Stop' : 'Run'}
  </Button>
}
```

**Removed complexity**:

- No `AbortController`
- No `WritableStream` creation
- No `pipeTo()` calls
- No `stdinWriter` / `TextEncoder`
- No `onDataHandlerRef` tracking
- No stderr timeout heuristics
- No duplicate handler prevention logic

---

## Part 5: Runtime stop() method

Unchanged from original — terminates worker and resolves promise:

```typescript
public stop(): void {
  if (this.currentWorker) {
    this.currentWorker.terminate();
    this.out.removeWorker(this.currentWorker);
    this.err.removeWorker(this.currentWorker);
    this.currentWorker = null;
    this.in.clear();
    if (this.stopResolver) {
      this.stopResolver();
      this.stopResolver = null;
    }
  }
}
```

---

## Part 6: Other features (unchanged)

These features from the original implementation remain:

- Terminal `enableInput()` / `disableInput()` via pointer-events
- Terminal `resize()` with FitAddon
- Draggable terminal resizer
- Worker error handling with timeout fallback
- Newline normalization (now built into `run()`)

---

## Files modified

| File                             | Changes                                                                             |
| -------------------------------- | ----------------------------------------------------------------------------------- |
| `src/ts/index.ts`                | Added `pushStdin()`, callback-based `run()`, `isRunning`, deprecated stream getters |
| `demo/hooks/useCodeExecution.ts` | **NEW** — extracted execution hook                                                  |
| `demo/components/Terminal.tsx`   | Added `onReady` callback prop                                                       |
| `demo/components/CodeEditor.tsx` | Simplified to use hook, added `terminalReady` state                                 |

---

## API comparison

| Aspect                | Old (Streams)                       | New (Callbacks)         |
| --------------------- | ----------------------------------- | ----------------------- |
| **Stdin**             | `stdin.getWriter().write(encoded)`  | `pushStdin(text)`       |
| **Stdout**            | `new WritableStream()` + `pipeTo()` | `run({ onStdout: fn })` |
| **Cleanup**           | `AbortController` + manual disposal | Automatic               |
| **Newlines**          | Manual `.replace(/\n/g, '\r\n')`    | Built into `run()`      |
| **Handler lifecycle** | Per-run setup/teardown              | Persistent `useEffect`  |

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    UNDERLYING MECHANISM                      │
│                      (always the same)                       │
├─────────────────────────────────────────────────────────────┤
│  Main Thread              SharedArrayBuffer           Worker │
│  pushStdin() ──write──►  [ring buffer]  ◄──read──  Atomics  │
│                          + Atomics.notify()         .wait() │
│                                                              │
│  onStdout() ◄──────────  postMessage()  ◄──────────  write  │
└─────────────────────────────────────────────────────────────┘
```

The streams were just a wrapper around `SharedArrayBuffer` + `Atomics` for stdin and `postMessage` listeners for stdout/stderr. We simplified the wrapper, not the mechanism.

---

## Line count comparison

| File                  | Before | After | Change                    |
| --------------------- | ------ | ----- | ------------------------- |
| `CodeEditor.tsx`      | 509    | 285   | **-224 lines (-44%)**     |
| `index.ts`            | 276    | 380   | +104 lines (new features) |
| `useCodeExecution.ts` | 0      | 159   | +159 lines (new file)     |
| `Terminal.tsx`        | 194    | 202   | +8 lines                  |

**Net**: More modular, easier to understand, complex stream plumbing hidden behind simple APIs.

---

## Features implemented

- Interactive stdin input from terminal
- Control sequences: Ctrl+C (stop), Ctrl+D (EOF), Ctrl+L (clear)
- Simplified callback-based I/O API
- Reusable `useCodeExecution` hook
- Terminal ready detection
- Proper UI state management
- Newline normalization (built-in)
- Scrollable code editor
- Draggable terminal/editor resizer
- Compilation error detection and handling
- Run/Stop button toggle
