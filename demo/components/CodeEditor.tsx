'use client';

import { cpp } from '@codemirror/lang-cpp';
import { oneDark } from '@codemirror/theme-one-dark';
import PlayArrowIcon from '@mui/icons-material/PlayArrow';
import StopIcon from '@mui/icons-material/Stop';
import { Box, Button, FormControl, InputLabel, MenuItem, Select, Typography } from '@mui/material';
import CodeMirror from '@uiw/react-codemirror';
import React, { useCallback, useEffect, useRef, useState } from 'react';

import Terminal, { TerminalHandle } from '@/components/Terminal';
import { useCodeExecution } from '@/hooks/useCodeExecution';

const defaultCode = `#include <iostream>

int main() {
  int x;
  std::cin >> x;
  std::cout << x << std::endl;
  return 0;
}`;

type Language = 'C' | 'C++';

export default function CodeEditor() {
  const [code, setCode] = useState<string>(defaultCode);
  const [language, setLanguage] = useState<Language>('C++');
  const [terminalHeight, setTerminalHeight] = useState<number>(170);
  const [terminalReady, setTerminalReady] = useState(false);
  const terminalRef = useRef<TerminalHandle | null>(null);
  const containerRef = useRef<HTMLDivElement | null>(null);
  const isDraggingRef = useRef<boolean>(false);

  // Use the simplified code execution hook
  const { isRunning, runCode, stopCode } = useCodeExecution({ terminalRef, terminalReady });

  const handleTerminalReady = useCallback(() => {
    setTerminalReady(true);
  }, []);

  const handleLanguageChange = (newLanguage: Language) => {
    setLanguage(newLanguage);
  };

  const handleRun = () => {
    runCode(code);
  };

  // Handle drag resizing of terminal
  useEffect(() => {
    const handleMouseMove = (e: MouseEvent) => {
      if (!isDraggingRef.current || !containerRef.current) return;

      const containerRect = containerRef.current.getBoundingClientRect();
      const containerHeight = containerRect.height;
      const mouseY = e.clientY;
      const relativeY = mouseY - containerRect.top;

      // Calculate new terminal height (from bottom)
      // Min height: 100px, Max height: containerHeight - 200px (leave room for editor)
      const newHeight = Math.max(100, Math.min(containerHeight - 200, containerHeight - relativeY));
      setTerminalHeight(newHeight);

      // Resize terminal immediately
      requestAnimationFrame(() => {
        terminalRef.current?.resize();
      });
    };

    const handleMouseUp = () => {
      if (isDraggingRef.current) {
        isDraggingRef.current = false;
        document.body.style.cursor = '';
        document.body.style.userSelect = '';
        // Final resize after drag ends
        requestAnimationFrame(() => {
          terminalRef.current?.resize();
        });
      }
    };

    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);

    return () => {
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
    };
  }, []);

  const handleMouseDown = (e: React.MouseEvent) => {
    e.preventDefault();
    isDraggingRef.current = true;
    document.body.style.cursor = 'row-resize';
    document.body.style.userSelect = 'none';
  };

  const extensions = [cpp()];

  return (
    <Box sx={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
      {/* Header */}
      <Box
        sx={{
          px: 3,
          py: 1.75,
          borderBottom: '1px solid',
          borderColor: 'rgba(148, 163, 184, 0.15)',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          gap: 3,
          background:
            'linear-gradient(180deg, rgba(18, 18, 24, 0.9) 0%, rgba(12, 12, 16, 0.6) 100%)',
          backdropFilter: 'blur(8px)',
        }}
      >
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 2, flexWrap: 'wrap' }}>
          <Box sx={{ display: 'flex', flexDirection: 'column', gap: 0.25 }}>
            <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
              <Typography variant="subtitle2" sx={{ fontWeight: 600 }}>
                Runtime Playground
              </Typography>
              <Box
                sx={{
                  px: 1,
                  py: 0.3,
                  borderRadius: 1,
                  fontSize: '0.65rem',
                  letterSpacing: '0.12em',
                  textTransform: 'uppercase',
                  color: '#c7d2fe',
                  background: 'rgba(99, 102, 241, 0.2)',
                  border: '1px solid rgba(99, 102, 241, 0.45)',
                }}
              >
                Demo
              </Box>
            </Box>
            <Typography variant="caption" sx={{ color: 'rgba(255, 255, 255, 0.55)' }}>
              Edit, run, and review output instantly
            </Typography>
          </Box>
          <FormControl size="small" sx={{ minWidth: 150 }}>
            <InputLabel sx={{ fontSize: '0.875rem' }}>Language</InputLabel>
            <Select
              value={language}
              label="Language"
              onChange={(e) => handleLanguageChange(e.target.value as Language)}
              sx={{
                fontSize: '0.875rem',
                '& .MuiOutlinedInput-notchedOutline': {
                  borderColor: 'rgba(148, 163, 184, 0.35)',
                },
                '&:hover .MuiOutlinedInput-notchedOutline': {
                  borderColor: 'rgba(148, 163, 184, 0.55)',
                },
              }}
            >
              <MenuItem value="C">C</MenuItem>
              <MenuItem value="C++">C++</MenuItem>
            </Select>
          </FormControl>
        </Box>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 2 }}>
          <Typography
            variant="caption"
            sx={{
              color: 'rgba(255, 255, 255, 0.55)',
              fontSize: '0.75rem',
              fontFamily:
                'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace',
              fontVariantNumeric: 'tabular-nums',
            }}
          >
            {code.split('\n').length} lines
          </Typography>

          {/* Run/Stop Button */}
          <Button
            variant="contained"
            size="small"
            startIcon={isRunning ? <StopIcon /> : <PlayArrowIcon />}
            onClick={isRunning ? stopCode : handleRun}
            sx={{
              minWidth: 100,
              textTransform: 'none',
              background: isRunning
                ? 'linear-gradient(135deg, #ef4444 0%, #dc2626 100%)'
                : 'linear-gradient(135deg, #6366f1 0%, #8b5cf6 100%)',
              border: '1px solid rgba(255, 255, 255, 0.12)',
              boxShadow: isRunning
                ? '0 10px 25px rgba(239, 68, 68, 0.3)'
                : '0 10px 25px rgba(99, 102, 241, 0.3)',
              '&:hover': {
                background: isRunning
                  ? 'linear-gradient(135deg, #dc2626 0%, #b91c1c 100%)'
                  : 'linear-gradient(135deg, #5855eb 0%, #7c3aed 100%)',
              },
            }}
          >
            {isRunning ? 'Stop' : 'Run'}
          </Button>
        </Box>
      </Box>

      {/* Main Content */}
      <Box
        ref={containerRef}
        sx={{
          flex: 1,
          overflow: 'hidden',
          display: 'flex',
          flexDirection: 'column',
          position: 'relative',
        }}
      >
        {/* Code Editor */}
        <Box sx={{ flex: 1, overflow: 'auto', background: 'rgba(10, 12, 18, 0.6)', minHeight: 0 }}>
          <CodeMirror
            value={code}
            height="100%"
            theme={oneDark}
            extensions={extensions}
            onChange={(value) => setCode(value)}
            basicSetup={{
              lineNumbers: true,
              foldGutter: true,
              dropCursor: false,
              allowMultipleSelections: false,
              indentOnInput: true,
              bracketMatching: true,
              closeBrackets: true,
              autocompletion: true,
              highlightSelectionMatches: true,
            }}
          />
        </Box>

        {/* Draggable Resizer */}
        <Box
          onMouseDown={handleMouseDown}
          sx={{
            height: '4px',
            cursor: 'row-resize',
            backgroundColor: 'rgba(148, 163, 184, 0.15)',
            position: 'relative',
            '&:hover': {
              backgroundColor: 'rgba(148, 163, 184, 0.3)',
            },
            '&::before': {
              content: '""',
              position: 'absolute',
              top: '-2px',
              left: 0,
              right: 0,
              height: '8px',
              cursor: 'row-resize',
            },
          }}
        />

        {/* Output Header */}
        <Box
          sx={{
            px: 2.5,
            py: 0.75,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            borderTop: '1px solid rgba(148, 163, 184, 0.15)',
            background: 'rgba(9, 11, 16, 0.75)',
            flexShrink: 0,
          }}
        >
          <Typography
            variant="caption"
            sx={{
              color: 'rgba(255, 255, 255, 0.55)',
              letterSpacing: '0.12em',
              textTransform: 'uppercase',
            }}
          >
            Output
          </Typography>
          <Typography
            variant="caption"
            sx={{ color: isRunning ? '#fbbf24' : 'rgba(255, 255, 255, 0.4)' }}
          >
            {isRunning ? 'Running' : 'Ready'}
          </Typography>
        </Box>

        {/* Terminal */}
        <Terminal ref={terminalRef} height={terminalHeight} onReady={handleTerminalReady} />
      </Box>
    </Box>
  );
}
