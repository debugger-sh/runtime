use std::{
    io,
    pin::Pin,
    task::{Context, Poll},
};

use wasmer_wasix::virtual_fs::{AsyncRead, AsyncSeek, AsyncWrite, Result, VirtualFile};
use web_sys::console;

#[derive(Debug, Default)]
pub struct ConsoleFile;

impl AsyncWrite for ConsoleFile {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        if let Ok(s) = std::str::from_utf8(buf) {
            console::log_1(&s.into());
        } else {
            console::log_1(&format!("{:?}", buf).into());
        }
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.poll_flush(cx)
    }
}

impl AsyncRead for ConsoleFile {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _buf: &mut wasmer_wasix::virtual_fs::ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

impl AsyncSeek for ConsoleFile {
    fn start_seek(self: Pin<&mut Self>, _position: io::SeekFrom) -> io::Result<()> {
        Ok(())
    }

    fn poll_complete(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<u64>> {
        Poll::Ready(Ok(0))
    }
}

impl VirtualFile for ConsoleFile {
    fn last_accessed(&self) -> u64 {
        0
    }
    fn last_modified(&self) -> u64 {
        0
    }
    fn created_time(&self) -> u64 {
        0
    }
    fn size(&self) -> u64 {
        0
    }

    fn set_len(&mut self, _new_size: u64) -> Result<()> {
        Ok(())
    }

    fn unlink(&mut self) -> Result<()> {
        Ok(())
    }

    fn get_special_fd(&self) -> Option<u32> {
        Some(1) // stdout
    }

    fn poll_read_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<usize>> {
        Poll::Ready(Ok(0))
    }

    fn poll_write_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<usize>> {
        Poll::Ready(Ok(usize::MAX))
    }
}
