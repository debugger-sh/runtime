'use client';

import { useCallback, useEffect, useRef, useState } from 'react';
import { Runtime } from 'runtime';

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
 * Hook to manage code execution with terminal I/O integration.
 *
 * Handles:
 * - Running code via the Runtime
 * - Piping stdout/stderr to the terminal
 * - Capturing stdin from terminal input
 * - Ctrl+C to stop execution
 * - Ctrl+D for EOF
 * - Ctrl+L to clear terminal
 *
 * @example
 * ```tsx
 * const terminalRef = useRef<TerminalHandle>(null);
 * const [terminalReady, setTerminalReady] = useState(false);
 * const { isRunning, runCode, stopCode } = useCodeExecution({ terminalRef, terminalReady });
 *
 * // In your run button handler:
 * await runCode(editorContent);
 * ```
 */
export function useCodeExecution({
  terminalRef,
  terminalReady,
}: UseCodeExecutionOptions): UseCodeExecutionResult {
  const [isRunning, setIsRunning] = useState(false);
  const runtimeRef = useRef<Runtime | null>(null);
  const isRunningRef = useRef(false); // For stdin handler to avoid stale closure

  // Set up persistent stdin handler (only when terminal is ready)
  useEffect(() => {
    if (!terminalReady) return;

    const terminal = terminalRef.current?.getTerminal();
    if (!terminal) return;

    let stdinBuffer = '';

    const onData = terminal.onData((data: string) => {
      // Only accept input when code is running
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
        // Enter: send buffered input with newline
        terminal.write('\r\n');
        runtimeRef.current.pushStdin(`${stdinBuffer}\n`);
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

      const rt = Runtime.create('c');
      runtimeRef.current = rt;
      rt.fs = { 'main.c': code };

      // Enable terminal input
      terminalRef.current?.enableInput();

      try {
        await rt.run({
          onStdout: (text) => terminalRef.current?.write(text),
          onStderr: (text) => terminalRef.current?.write(text),
        });
      } catch (error) {
        console.error('Execution error:', error);
        terminalRef.current?.writeln(
          `\r\nError: ${error instanceof Error ? error.message : String(error)}`
        );
      } finally {
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
    stopCode,
  };
}
