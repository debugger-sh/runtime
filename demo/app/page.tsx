'use client';

import { Box, Paper, Typography } from '@mui/material';
import CssBaseline from '@mui/material/CssBaseline';
import { createTheme, ThemeProvider } from '@mui/material/styles';

import CodeEditor from '@/components/CodeEditor';

const darkTheme = createTheme({
  palette: {
    mode: 'dark',
    primary: {
      main: '#6366f1',
    },
    background: {
      default: '#0a0a0a',
      paper: '#1a1a1a',
    },
  },
  shape: {
    borderRadius: 2,
  },
});

export default function Home() {
  return (
    <ThemeProvider theme={darkTheme}>
      <CssBaseline />
      <Box
        sx={{
          minHeight: '100vh',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          p: { xs: 2, md: 4 },
          position: 'relative',
          overflow: 'hidden',
          background: 'radial-gradient(1200px 800px at 15% 10%, #111827 0%, #050505 55%)',
        }}
      >
        <Box
          sx={{
            position: 'absolute',
            inset: 0,
            background:
              'radial-gradient(900px 500px at 80% 20%, rgba(59, 130, 246, 0.18), transparent 60%), radial-gradient(700px 420px at 20% 70%, rgba(99, 102, 241, 0.18), transparent 65%)',
            opacity: 0.9,
            pointerEvents: 'none',
          }}
        />
        <Box sx={{ position: 'relative', width: '100%', maxWidth: '1080px' }}>
          <Box sx={{ textAlign: 'center', mb: 3 }}>
            {/* <Typography
              variant="h4"
              sx={{ fontWeight: 700, color: 'rgba(255, 255, 255, 0.92)', letterSpacing: '-0.02em' }}
            >
              Runtime Playground
            </Typography> */}
            <Typography variant="body2" sx={{ color: 'rgba(255, 255, 255, 0.6)', mt: 1 }}>
              Write, run, and inspect code output in one sleek panel.
            </Typography>
          </Box>
          <Box
            sx={{
              p: '1px',
              borderRadius: 2,
              background:
                'linear-gradient(135deg, rgba(99, 102, 241, 0.65) 0%, rgba(56, 189, 248, 0.3) 45%, rgba(236, 72, 153, 0.35) 100%)',
            }}
          >
            <Paper
              elevation={0}
              sx={{
                width: '100%',
                height: { xs: 560, md: 640 },
                display: 'flex',
                flexDirection: 'column',
                overflow: 'hidden',
                borderRadius: 2,
                background: 'rgba(10, 12, 18, 0.92)',
                border: '1px solid rgba(148, 163, 184, 0.18)',
                boxShadow: '0 30px 90px rgba(0, 0, 0, 0.55)',
                backdropFilter: 'blur(12px)',
              }}
            >
              <CodeEditor />
            </Paper>
          </Box>
        </Box>
      </Box>
    </ThemeProvider>
  );
}
