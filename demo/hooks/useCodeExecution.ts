'use client';

import { Runtime } from '@jtrb/runtime';
import { useCallback, useEffect, useRef, useState } from 'react';

import type { TerminalHandle } from '@/components/Terminal';

type UseCodeExecutionOptions = {
  /** Reference to the terminal component */
  terminalRef: React.RefObject<TerminalHandle | null>;
  /** Whether the terminal is ready (xterm initialized) */
  terminalReady: boolean;
};

type UseCodeExecutionResult = {
  /** Whether code is currently running */
  isRunning: boolean;
  /** Run the provided code */
  runCode: (code: string) => Promise<void>;
  /** Stop the currently running code */
  stopCode: () => void;
};

/**
 * Hook to manage code execution with terminal I/O.
 *
 * Handles:
 * - Running code via the Runtime
 * - Subscribing to stdout/stderr and printing to the terminal
 * - Sending stdin from terminal input via `rt.stdin.write`
 * - Ctrl+C to stop execution
 * - Ctrl+D for EOF
 * - Ctrl+L to clear terminal
 */
export function useCodeExecution({
  terminalRef,
  terminalReady
}: UseCodeExecutionOptions): UseCodeExecutionResult {
  const [isRunning, setIsRunning] = useState(false);
  const runtimeRef = useRef<Runtime | null>(null);
  const isRunningRef = useRef(false);
  const encoderRef = useRef(new TextEncoder());

  // Set up persistent stdin handler (only when terminal is ready)
  useEffect(() => {
    if (!terminalReady) return;

    const terminal = terminalRef.current?.getTerminal();
    if (!terminal) return;

    let stdinBuffer = '';

    const onData = terminal.onData((data: string) => {
      // Only accept input when code is running
      if (!isRunningRef.current || !runtimeRef.current) return;

      const encoder = encoderRef.current;
      const stdin = runtimeRef.current.stdin;

      // Control sequences
      if (data === '\x03') {
        // Ctrl+C: Stop execution
        terminal.write('^C\r\n');
        runtimeRef.current?.stop();
        return;
      } else if (data === '\x04') {
        // Ctrl+D: Send EOF
        terminal.write('^D\r\n');
        void stdin.write(encoder.encode('\x04'));
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
        // Enter: send buffered input with newline
        terminal.write('\r\n');
        void stdin.write(encoder.encode(`${stdinBuffer}\n`));
        stdinBuffer = '';
      } else if (data === '\u007f') {
        // Backspace: remove last character
        if (stdinBuffer.length > 0) {
          stdinBuffer = stdinBuffer.slice(0, -1);
          terminal.write('\b \b');
        }
      } else if (
        // Ignore arrow keys
        data === '\x1b[A' ||
        data === '\x1b[B' ||
        data === '\x1b[C' ||
        data === '\x1b[D'
      ) {
        return;
      } else {
        // Regular character: add to buffer and echo
        stdinBuffer += data;
        terminal.write(data);
      }
    });

    return () => {
      onData.dispose();
    };
  }, [terminalRef, terminalReady]);

  const runCode = useCallback(
    async (code: string) => {
      if (isRunningRef.current) return; // Already running

      setIsRunning(true);
      isRunningRef.current = true;

      // Clear terminal for fresh output
      terminalRef.current?.clear();

      const rt = await Runtime.create('c');
      runtimeRef.current = rt;
      rt.fs = { 'main.c': code };

      const decoder = new TextDecoder();
      const onIo = (chunk: Uint8Array) => {
        const text = decoder.decode(chunk);
        const normalized = text.replace(/\r?\n/g, '\r\n');
        terminalRef.current?.write(normalized);
      };
      rt.stdout.on('data', onIo);
      rt.stderr.on('data', onIo);

      // Enable terminal input
      terminalRef.current?.enableInput();

      try {
        await rt.run();
      } catch (error) {
        console.error('Execution error:', error);
        terminalRef.current?.writeln(
          `\r\nError: ${error instanceof Error ? error.message : String(error)}`
        );
      } finally {
        rt.stdout.off('data', onIo);
        rt.stderr.off('data', onIo);
        runtimeRef.current = null;
        isRunningRef.current = false;
        setIsRunning(false);
        terminalRef.current?.disableInput();
      }
    },
    [terminalRef]
  );

  const stopCode = useCallback(() => {
    runtimeRef.current?.stop();
  }, []);

  return {
    isRunning,
    runCode,
    stopCode
  };
}
