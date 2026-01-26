import { StdoutMode, WorkerOut, WorkerStart } from '../../pkg/runtime';
import RustWorker from './worker?worker&inline';

export type Lang = 'c';

// TODO: Find a way to re-use the generated types in `pkg/runtime.d.ts`
export type FsNode = string | DirNode;
export type DirNode = { [name: string]: FsNode };

/**
 * Options for running code with callback-based I/O.
 */
export type RunOptions = {
  /** Called when the program writes to stdout. Receives normalized text (with \r\n line endings). */
  onStdout?: (data: string) => void;
  /** Called when the program writes to stderr. Receives normalized text (with \r\n line endings). */
  onStderr?: (data: string) => void;
  /** AbortSignal to cancel execution. */
  signal?: AbortSignal;
};

export class Runtime {
  private out = new StdoutStream(1);
  private err = new StdoutStream(2);
  private in = new StdinStream();
  private currentWorker: Worker | null = null;
  private stopResolver: ((value: void) => void) | null = null;
  private decoder = new TextDecoder();
  private encoder = new TextEncoder();

  /**
   * The programming language of this runtime.
   */
  public readonly lang: Lang;

  /**
   * The *initial* filesystem that the code sees.
   *
   * This is neither updated while the code is running, nor
   * will updating it have any effect on code that is already running.
   */
  public fs: DirNode = {};

  /**
   * A [WritableStream](https://developer.mozilla.org/en-US/docs/Web/API/WritableStream) for writing to the program's `stdin` (fd 0).
   *
   * Note that any previous input pushed to `stdin` will be cleared when the program finishes
   * running. This is to prevent subsequent runs of a program from seeing `stdin` from the previous one.
   *
   * @example
   * ```ts
   *  const rt = Runtime.create('c');
   *
   *  const encoder = new TextEncoder();
   *  const writer = rt.stdin.getWriter();
   *
   *  writer.write(encoder.encode('hello world\n'));
   * ```
   *
   * @deprecated Use `pushStdin()` for simpler API
   */
  public get stdin() {
    return this.in.stream;
  }

  /**
   * A [ReadableStream](https://developer.mozilla.org/en-US/docs/Web/API/ReadableStream) for reading the program's `stdout` (fd 1).
   *
   * @deprecated Use `run({ onStdout })` for simpler API
   */
  public get stdout() {
    return this.out.stream;
  }

  /**
   * A [ReadableStream](https://developer.mozilla.org/en-US/docs/Web/API/ReadableStream) for reading the program's `stderr` (fd 2).
   *
   * @deprecated Use `run({ onStderr })` for simpler API
   */
  public get stderr() {
    return this.err.stream;
  }

  /**
   * Whether code is currently running.
   */
  public get isRunning(): boolean {
    return this.currentWorker !== null;
  }

  static create(lang: Lang): Runtime {
    return new Runtime(lang);
  }

  private constructor(lang: Lang) {
    this.lang = lang;
  }

  /**
   * Push text to the program's stdin.
   * This is the simplest way to send input to a running program.
   *
   * @example
   * ```ts
   * const rt = Runtime.create('c');
   * // ... in your input handler:
   * rt.pushStdin('hello world\n');
   * ```
   */
  public pushStdin(text: string): void {
    const encoded = this.encoder.encode(text);
    this.in.writeSync(encoded);
  }

  /**
   * Stops the currently running execution by terminating the worker.
   * Safe to call even if no execution is running.
   * This will cause the run() promise to resolve immediately.
   */
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

  /**
   * Run the code in the filesystem.
   *
   * @param options - Optional callbacks for stdout/stderr and abort signal
   *
   * @example
   * ```ts
   * const rt = Runtime.create('c');
   * rt.fs = { 'main.c': code };
   *
   * await rt.run({
   *   onStdout: (text) => terminal.write(text),
   *   onStderr: (text) => terminal.write(text),
   *   signal: abortController.signal,
   * });
   * ```
   */
  async run(options?: RunOptions): Promise<void> {
    const { onStdout, onStderr, signal } = options ?? {};

    // Handle abort signal
    if (signal?.aborted) {
      return;
    }

    const abortHandler = () => this.stop();
    signal?.addEventListener('abort', abortHandler);

    const worker = new RustWorker();
    this.currentWorker = worker;

    // Set up callback-based stdout/stderr handling if provided
    const stdoutCallback = onStdout
      ? (event: MessageEvent<WorkerOut>) => {
          const msg = event.data;
          if (msg.type !== 'stdout' || msg.mode !== 1) return;
          const text = this.decoder.decode(msg.data as Uint8Array);
          onStdout(text.replaceAll('\n', '\r\n'));
        }
      : null;

    const stderrCallback = onStderr
      ? (event: MessageEvent<WorkerOut>) => {
          const msg = event.data;
          if (msg.type !== 'stdout' || msg.mode !== 2) return;
          const text = this.decoder.decode(msg.data as Uint8Array);
          onStderr(text.replaceAll('\n', '\r\n'));
        }
      : null;

    if (stdoutCallback) worker.addEventListener('message', stdoutCallback);
    if (stderrCallback) worker.addEventListener('message', stderrCallback);

    // Also set up stream-based handling (for backwards compatibility)
    this.out.addWorker(worker);
    this.err.addWorker(worker);

    /* Wait for the worker to send us a Ready message */
    await new Promise<void>((resolve) => {
      const callback = (message: MessageEvent<WorkerOut>) => {
        if (message.data.type === 'ready') {
          worker.removeEventListener('message', callback);
          resolve();
        }
      };
      worker.addEventListener('message', callback);
    });

    /**
     * Run the worker, and wait for it to send us a Stop message.
     */
    const stop = new Promise<void>((resolve) => {
      this.stopResolver = resolve;
      let resolved = false;
      let maxTimeout: ReturnType<typeof setTimeout> | null = null;

      const doResolve = () => {
        if (resolved) return;
        resolved = true;
        if (maxTimeout) {
          clearTimeout(maxTimeout);
          maxTimeout = null;
        }
        worker.removeEventListener('message', callback);
        worker.removeEventListener('error', errorCallback);
        this.stopResolver = null;
        resolve();
      };

      const errorCallback = (error: ErrorEvent) => {
        console.error('Worker error:', error);
        doResolve();
      };

      const callback = (message: MessageEvent<WorkerOut>) => {
        if (message.data.type === 'stop') {
          doResolve();
        }
      };

      worker.addEventListener('message', callback);
      worker.addEventListener('error', errorCallback);

      // Set a maximum timeout (30 seconds) to prevent hanging forever
      maxTimeout = setTimeout(() => {
        if (!resolved) {
          console.warn('Worker timeout - resolving promise (worker may have panicked)');
          doResolve();
        }
      }, 30000);
    });

    const message: WorkerStart = {
      fs: this.fs,
      stdin_buffer: this.in.buffer,
      is_debug: true,
    };
    worker.postMessage(message);

    await stop;

    // Cleanup
    signal?.removeEventListener('abort', abortHandler);
    if (stdoutCallback) worker.removeEventListener('message', stdoutCallback);
    if (stderrCallback) worker.removeEventListener('message', stderrCallback);
    this.out.removeWorker(worker);
    this.err.removeWorker(worker);
    this.in.clear();
    this.currentWorker = null;
    this.stopResolver = null;
  }
}

class StdoutStream {
  public readonly stream: ReadableStream<Uint8Array<ArrayBuffer>>;
  private controller?: ReadableStreamDefaultController<Uint8Array<ArrayBuffer>>;
  private callback: (event: MessageEvent<WorkerOut>) => void;

  constructor(public readonly mode: StdoutMode) {
    this.stream = new ReadableStream({
      start: (controller) => (this.controller = controller),
    });

    this.callback = ((event: MessageEvent<WorkerOut>) => {
      const msg = event.data;
      if (msg.type !== 'stdout') return;
      if (msg.mode !== this.mode) return;
      this.controller?.enqueue(msg.data as Uint8Array<ArrayBuffer>);
    }).bind(this);
  }

  public addWorker(worker: Worker) {
    worker.addEventListener('message', this.callback);
  }

  public removeWorker(worker: Worker) {
    worker.removeEventListener('message', this.callback);
  }
}

class StdinStream {
  /**
   * Ring buffer to store stdin data.
   *
   * - TypeScript controls write_index, Rust controls read_index
   * - One slot is always kept empty to distinguish full from empty
   */

  private static readonly BUFFER_SIZE = 16;
  private static readonly HEADER_SIZE = 8; // 2 x i32
  private static readonly DATA_SIZE = StdinStream.BUFFER_SIZE - StdinStream.HEADER_SIZE;
  private static readonly READ_IDX = 0;
  private static readonly WRITE_IDX = 1;

  public readonly buffer = new SharedArrayBuffer(StdinStream.BUFFER_SIZE);
  public readonly stream: WritableStream<Uint8Array>;

  private readonly indices: Int32Array;
  private readonly data: Int8Array;

  constructor() {
    this.stream = new WritableStream({
      write: (chunk) => this.write(chunk),
    });
    this.indices = new Int32Array(this.buffer, 0, 2);
    this.data = new Int8Array(this.buffer, StdinStream.HEADER_SIZE);
  }

  public clear() {
    this.indices.fill(0);
  }

  /**
   * Synchronously write data to the stdin buffer.
   * Used by Runtime.pushStdin() for simpler API.
   */
  public writeSync(chunk: Uint8Array): void {
    const { DATA_SIZE, READ_IDX, WRITE_IDX } = StdinStream;
    let offset = 0;

    while (offset < chunk.length) {
      const readIdx = Atomics.load(this.indices, READ_IDX);
      let writeIdx = Atomics.load(this.indices, WRITE_IDX);

      if (writeIdx === DATA_SIZE - 1 && readIdx > 0) writeIdx = 0;
      const available = readIdx <= writeIdx ? DATA_SIZE - writeIdx - 1 : readIdx - writeIdx - 1;

      if (available === 0) {
        // Buffer full - in sync mode, we spin-wait (blocking)
        // This is acceptable for interactive input which is typically small
        continue;
      }

      const toWrite = Math.min(chunk.length - offset, available);
      this.data.set(chunk.subarray(offset, offset + toWrite), writeIdx);

      Atomics.store(this.indices, WRITE_IDX, (writeIdx + toWrite) % DATA_SIZE);
      Atomics.notify(this.indices, WRITE_IDX);
      offset += toWrite;
    }
  }

  private async write(chunk: Uint8Array): Promise<void> {
    const { DATA_SIZE, READ_IDX, WRITE_IDX } = StdinStream;
    let offset = 0;

    while (offset < chunk.length) {
      const readIdx = Atomics.load(this.indices, READ_IDX);
      let writeIdx = Atomics.load(this.indices, WRITE_IDX);

      if (writeIdx === DATA_SIZE - 1 && readIdx > 0) writeIdx = 0;
      const available = readIdx <= writeIdx ? DATA_SIZE - writeIdx - 1 : readIdx - writeIdx - 1;

      if (available === 0) {
        await Atomics.waitAsync(this.indices, READ_IDX, readIdx).value;
        continue;
      }

      const toWrite = Math.min(chunk.length - offset, available);
      this.data.set(chunk.subarray(offset, offset + toWrite), writeIdx);

      Atomics.store(this.indices, WRITE_IDX, (writeIdx + toWrite) % DATA_SIZE);
      Atomics.notify(this.indices, WRITE_IDX);
      offset += toWrite;
    }
  }
}
