/// DuplexSocket - Combines split AsyncRead + AsyncWrite halves
/// 
/// Problem: tokio TcpStream.into_split() gives ReadHalf and WriteHalf
/// These can't be recombined, and TLS acceptor needs a full AsyncRead+AsyncWrite
///
/// Solution: Wrapper that implements both traits by delegating to the halves
/// This allows us to accept TLS from the client after we've already split the socket!

use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

/// Combines split socket halves back into something that implements both AsyncRead and AsyncWrite
pub struct DuplexSocket<R: AsyncRead + Unpin, W: AsyncWrite + Unpin> {
    read: R,
    write: W,
}

impl<R: AsyncRead + Unpin, W: AsyncWrite + Unpin> DuplexSocket<R, W> {
    /// Create a new DuplexSocket from separate read and write halves
    pub fn new(read: R, write: W) -> Self {
        Self { read, write }
    }
}

impl<R: AsyncRead + Unpin, W: AsyncWrite + Unpin> AsyncRead for DuplexSocket<R, W> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.read).poll_read(cx, buf)
    }
}

impl<R: AsyncRead + Unpin, W: AsyncWrite + Unpin> AsyncWrite for DuplexSocket<R, W> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.write).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.write).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.write).poll_shutdown(cx)
    }
}

