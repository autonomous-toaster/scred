// C wrapper to properly export symbols for LD_PRELOAD
#include <unistd.h>
#include <stdio.h>
#include <dlfcn.h>

// Import the Rust functions
extern ssize_t rust_write(int fd, const void *buf, size_t count);
extern ssize_t rust_writev(int fd, const struct iovec *iov, int iovcnt);

// Export C symbols that LD_PRELOAD can find
ssize_t write(int fd, const void *buf, size_t count) {
    return rust_write(fd, buf, count);
}

ssize_t writev(int fd, const struct iovec *iov, int iovcnt) {
    return rust_writev(fd, iov, iovcnt);
}
