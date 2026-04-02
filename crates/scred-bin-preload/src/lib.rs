use libc::{c_int, c_void, size_t, ssize_t};
use redhook::{hook, real};
use std::cell::Cell;
use std::cmp::min;
use std::sync::{Mutex, OnceLock};

const LOOKAHEAD: usize = 512;
const MAX_FD: usize = 1024;

thread_local! {
    static IN_HOOK: Cell<bool> = const { Cell::new(false) };
}

struct FdState {
    tail: Vec<u8>,
}

impl FdState {
    fn new() -> Self {
        Self { tail: Vec::new() }
    }
}

fn states() -> &'static Vec<Mutex<FdState>> {
    static STATES: OnceLock<Vec<Mutex<FdState>>> = OnceLock::new();
    STATES.get_or_init(|| (0..MAX_FD).map(|_| Mutex::new(FdState::new())).collect())
}

fn active() -> bool {
    std::env::var_os("SCRED_BIN_ACTIVE").is_some()
}

fn debug_hooks() -> bool {
    std::env::var("SCRED_BIN_DEBUG_HOOKS").ok().as_deref() == Some("1")
}

fn debug_log(msg: &str) {
    if !debug_hooks() {
        return;
    }
    let _ = unsafe { real!(write)(2, msg.as_ptr() as *const c_void, msg.len()) };
}

fn tail_len(fd: c_int) -> usize {
    if fd < 0 || fd as usize >= MAX_FD {
        return 0;
    }
    states()[fd as usize].lock().unwrap().tail.len()
}

fn hook_stdout() -> bool {
    std::env::var("SCRED_BIN_HOOK_STDOUT").ok().as_deref() != Some("0")
}

fn hook_stderr() -> bool {
    std::env::var("SCRED_BIN_HOOK_STDERR").ok().as_deref() != Some("0")
}

fn hook_network() -> bool {
    std::env::var("SCRED_BIN_HOOK_NETWORK").ok().as_deref() == Some("1")
}

fn should_hook_fd(fd: c_int) -> bool {
    match fd {
        1 => hook_stdout(),
        2 => hook_stderr(),
        _ => false,
    }
}

fn redact_stream_bytes(fd: c_int, buf: &[u8]) -> Vec<u8> {
    if fd < 0 || fd as usize >= MAX_FD {
        return redact_whole(buf);
    }

    let state = &states()[fd as usize];
    let mut state = state.lock().unwrap();

    let mut combined = std::mem::take(&mut state.tail);
    combined.extend_from_slice(buf);

    let emit_len = combined.len().saturating_sub(LOOKAHEAD);
    if emit_len > 0 {
        let chunk = redact_whole(&combined[..emit_len]);
        state.tail = combined[emit_len..].to_vec();
        chunk
    } else {
        state.tail = combined;
        Vec::new()
    }
}

fn flush_stream_bytes(fd: c_int) -> Vec<u8> {
    if fd < 0 || fd as usize >= MAX_FD {
        return Vec::new();
    }
    let state = &states()[fd as usize];
    let mut state = state.lock().unwrap();
    let tail = std::mem::take(&mut state.tail);
    if tail.is_empty() {
        Vec::new()
    } else {
        redact_whole(&tail)
    }
}

fn redact_whole(buf: &[u8]) -> Vec<u8> {
    let detection = scred_detector::detect_all(buf);
    if detection.matches.is_empty() {
        return buf.to_vec();
    }
    let mut out = buf.to_vec();
    scred_detector::redact_in_place(&mut out, &detection.matches);
    out
}

fn flush_all_fds() {
    for fd in [1, 2] {
        let tail = flush_stream_bytes(fd);
        if !tail.is_empty() {
            let _ = write_all_real(fd, &tail);
        }
    }
}

fn flush_socket_tail(fd: c_int) {
    let tail = flush_stream_bytes(fd);
    if debug_hooks() {
        let msg = format!("[scred-bin] flush-socket fd={} bytes={}\n", fd, tail.len());
        debug_log(&msg);
    }
    if tail.is_empty() {
        return;
    }
    let _ = unsafe { real!(send)(fd, tail.as_ptr() as *const c_void, tail.len(), 0) };
}

fn write_all_real(fd: c_int, mut buf: &[u8]) -> ssize_t {
    let mut total = 0isize;
    while !buf.is_empty() {
        let written = unsafe { real!(write)(fd, buf.as_ptr() as *const c_void, buf.len()) };
        if written <= 0 {
            return if total > 0 { total } else { written };
        }
        total += written;
        buf = &buf[written as usize..];
    }
    total
}

fn enter_hook() -> bool {
    IN_HOOK.with(|flag| {
        let was = flag.get();
        if !was {
            flag.set(true);
        }
        !was
    })
}

fn leave_hook() {
    IN_HOOK.with(|flag| flag.set(false));
}

fn process_write_like(fd: c_int, input: &[u8]) -> Option<ssize_t> {
    if !active() || !should_hook_fd(fd) || !enter_hook() {
        return None;
    }

    let redacted = redact_stream_bytes(fd, input);
    if debug_hooks() {
        let msg = format!(
            "[scred-bin] fd-hook fd={} in={} emitted={} buffered={}\n",
            fd,
            input.len(),
            redacted.len(),
            tail_len(fd)
        );
        debug_log(&msg);
    }
    let result = if redacted.is_empty() {
        input.len() as ssize_t
    } else {
        let n = write_all_real(fd, &redacted);
        if n < 0 {
            n
        } else {
            input.len() as ssize_t
        }
    };

    leave_hook();
    Some(result)
}

fn process_network_like(
    fd: c_int,
    input: &[u8],
    send_fn: impl FnOnce(&[u8]) -> ssize_t,
) -> Option<ssize_t> {
    if !active() || !hook_network() || !enter_hook() {
        return None;
    }

    let redacted = redact_stream_bytes(fd, input);
    if debug_hooks() {
        let msg = format!(
            "[scred-bin] net-hook fd={} in={} emitted={} buffered={}\n",
            fd,
            input.len(),
            redacted.len(),
            tail_len(fd)
        );
        debug_log(&msg);
    }
    let result = if redacted.is_empty() {
        input.len() as ssize_t
    } else {
        let n = send_fn(&redacted);
        if n < 0 {
            n
        } else {
            input.len() as ssize_t
        }
    };

    leave_hook();
    Some(result)
}

fn process_sendmsg_payload(msg: &libc::msghdr) -> Vec<u8> {
    if msg.msg_iov.is_null() || msg.msg_iovlen == 0 {
        return Vec::new();
    }
    let iovs = unsafe { std::slice::from_raw_parts(msg.msg_iov, msg.msg_iovlen as usize) };
    let total_in: usize = iovs.iter().map(|v| v.iov_len).sum();
    let mut combined = Vec::with_capacity(min(total_in, 1 << 20));
    for v in iovs {
        if !v.iov_base.is_null() && v.iov_len > 0 {
            let part = unsafe { std::slice::from_raw_parts(v.iov_base as *const u8, v.iov_len) };
            combined.extend_from_slice(part);
        }
    }
    combined
}

fn build_iovec(buf: &[u8]) -> libc::iovec {
    libc::iovec {
        iov_base: buf.as_ptr() as *mut c_void,
        iov_len: buf.len(),
    }
}

hook! {
    unsafe fn write(fd: c_int, buf: *const c_void, count: size_t) -> ssize_t => hooked_write {

        if debug_hooks() {
            let msg = format!("[scred-bin] write \n");
            debug_log(&msg);
        }

        if !active() || buf.is_null() || count == 0 {
            return real!(write)(fd, buf, count);
        }
        let input = std::slice::from_raw_parts(buf as *const u8, count);
        process_write_like(fd, input).unwrap_or_else(|| real!(write)(fd, buf, count))
    }
}

#[allow(non_snake_case)]
hook! {
    unsafe fn __write_chk(fd: c_int, buf: *const c_void, count: size_t, buflen: size_t) -> ssize_t => hooked___write_chk {
        if debug_hooks() {
            let msg = format!("[scred-bin] __write_chk \n");
            debug_log(&msg);
        }
        if !active() || buf.is_null() || count == 0 {
            return real!(__write_chk)(fd, buf, count, buflen);
        }
        let input = std::slice::from_raw_parts(buf as *const u8, count);
        process_write_like(fd, input).unwrap_or_else(|| real!(__write_chk)(fd, buf, count, buflen))
    }
}

hook! {
    unsafe fn writev(fd: c_int, iov: *const libc::iovec, iovcnt: c_int) -> ssize_t => hooked_writev {
        if debug_hooks() {
            let msg = format!("[scred-bin] writev \n");
            debug_log(&msg);
        }
        if !active() || iov.is_null() || iovcnt <= 0 {
            return real!(writev)(fd, iov, iovcnt);
        }

        let iovs = std::slice::from_raw_parts(iov, iovcnt as usize);
        let total_in: usize = iovs.iter().map(|v| v.iov_len).sum();
        let mut combined = Vec::with_capacity(min(total_in, 1 << 20));
        for v in iovs {
            if !v.iov_base.is_null() && v.iov_len > 0 {
                let part = std::slice::from_raw_parts(v.iov_base as *const u8, v.iov_len);
                combined.extend_from_slice(part);
            }
        }

        process_write_like(fd, &combined).unwrap_or_else(|| real!(writev)(fd, iov, iovcnt))
    }
}

hook! {
    unsafe fn puts(s: *const libc::c_char) -> c_int => hooked_puts {
        if debug_hooks() {
            let msg = format!("[scred-bin] puts \n");
            debug_log(&msg);
        }
        if !active() || s.is_null() {
            return real!(puts)(s);
        }
        let len = libc::strlen(s) as usize;
        let bytes = std::slice::from_raw_parts(s as *const u8, len);
        let mut data = Vec::with_capacity(len + 1);
        data.extend_from_slice(bytes);
        data.push(b'\n');
        process_write_like(1, &data).map(|_| 1 as c_int).unwrap_or_else(|| real!(puts)(s))
    }
}

hook! {
    unsafe fn fputs(s: *const libc::c_char, stream: *mut libc::FILE) -> c_int => hooked_fputs {
        if debug_hooks() {
            let msg = format!("[scred-bin] fputs \n");
            debug_log(&msg);
        }    
        if !active() || s.is_null() || stream.is_null() {
            return real!(fputs)(s, stream);
        }
        let fd = libc::fileno(stream);
        if fd < 0 {
            return real!(fputs)(s, stream);
        }
        let len = libc::strlen(s) as usize;
        let bytes = std::slice::from_raw_parts(s as *const u8, len);
        process_write_like(fd, bytes).map(|_| 1 as c_int).unwrap_or_else(|| real!(fputs)(s, stream))
    }
}

hook! {
    unsafe fn fwrite(ptr: *const c_void, size: size_t, nmemb: size_t, stream: *mut libc::FILE) -> size_t => hooked_fwrite {
        if debug_hooks() {
            let msg = format!("[scred-bin] fwrite \n");
            debug_log(&msg);
        }
        if !active() || ptr.is_null() || size == 0 || nmemb == 0 || stream.is_null() {
            return real!(fwrite)(ptr, size, nmemb, stream);
        }
        let fd = libc::fileno(stream);
        if fd < 0 {
            return real!(fwrite)(ptr, size, nmemb, stream);
        }
        let total = size.saturating_mul(nmemb);
        let input = std::slice::from_raw_parts(ptr as *const u8, total);
        process_write_like(fd, input).map(|_| nmemb).unwrap_or_else(|| real!(fwrite)(ptr, size, nmemb, stream))
    }
}

hook! {
    unsafe fn close(fd: c_int) -> c_int => hooked_close {
        if active() && enter_hook() {
            if debug_hooks() {
                let msg = format!("[scred-bin] close fd={}\n", fd);
                debug_log(&msg);
            }
            if should_hook_fd(fd) {
                if debug_hooks() {
                    let msg = format!("[scred-bin] should_hook_fd \n");
                    debug_log(&msg);
                }
                let tail = flush_stream_bytes(fd);
                if !tail.is_empty() {
                    let _ = write_all_real(fd, &tail);
                }
            } else if hook_network() {
                flush_socket_tail(fd);
            }
            leave_hook();
        }
        real!(close)(fd)
    }
}

hook! {
    unsafe fn fflush(stream: *mut libc::FILE) -> c_int => hooked_fflush {
        if active() && enter_hook() {
            flush_all_fds();
            leave_hook();
        }
        real!(fflush)(stream)
    }
}

hook! {
    unsafe fn exit(status: c_int) -> ! => hooked_exit {
        if active() && enter_hook() {
            flush_all_fds();
            leave_hook();
        }
        real!(exit)(status)
    }
}

hook! {
    unsafe fn shutdown(sockfd: c_int, how: c_int) -> c_int => hooked_shutdown {
        if active() && hook_network() && enter_hook() {
            if debug_hooks() {
                let msg = format!("[scred-bin] shutdown fd={} how={}\n", sockfd, how);
                debug_log(&msg);
            }
            flush_socket_tail(sockfd);
            leave_hook();
        }
        real!(shutdown)(sockfd, how)
    }
}

hook! {
    unsafe fn connect(sockfd: c_int, addr: *const libc::sockaddr, addrlen: libc::socklen_t) -> c_int => hooked_connect {
        if active() && hook_network() && debug_hooks() && !addr.is_null() {
            let family = (*addr).sa_family as i32;
            let msg = format!("[scred-bin] connect fd={} family={} addrlen={}\n", sockfd, family, addrlen);
            debug_log(&msg);
        }
        real!(connect)(sockfd, addr, addrlen)
    }
}

hook! {
    unsafe fn send(sockfd: c_int, buf: *const c_void, len: size_t, flags: c_int) -> ssize_t => hooked_send {
        if !active() || !hook_network() || buf.is_null() || len == 0 {
            return real!(send)(sockfd, buf, len, flags);
        }
        let input = std::slice::from_raw_parts(buf as *const u8, len);
        process_network_like(sockfd, input, |redacted| unsafe {
            real!(send)(sockfd, redacted.as_ptr() as *const c_void, redacted.len(), flags)
        }).unwrap_or_else(|| real!(send)(sockfd, buf, len, flags))
    }
}

hook! {
    unsafe fn sendto(sockfd: c_int, buf: *const c_void, len: size_t, flags: c_int, dest_addr: *const libc::sockaddr, addrlen: libc::socklen_t) -> ssize_t => hooked_sendto {
        if !active() || !hook_network() || buf.is_null() || len == 0 {
            return real!(sendto)(sockfd, buf, len, flags, dest_addr, addrlen);
        }
        let input = std::slice::from_raw_parts(buf as *const u8, len);
        process_network_like(sockfd, input, |redacted| unsafe {
            real!(sendto)(sockfd, redacted.as_ptr() as *const c_void, redacted.len(), flags, dest_addr, addrlen)
        }).unwrap_or_else(|| real!(sendto)(sockfd, buf, len, flags, dest_addr, addrlen))
    }
}

hook! {
    unsafe fn sendmsg(sockfd: c_int, msg: *const libc::msghdr, flags: c_int) -> ssize_t => hooked_sendmsg {
        if !active() || !hook_network() || msg.is_null() {
            return real!(sendmsg)(sockfd, msg, flags);
        }
        let payload = process_sendmsg_payload(&*msg);
        if payload.is_empty() {
            return real!(sendmsg)(sockfd, msg, flags);
        }
        process_network_like(sockfd, &payload, |redacted| unsafe {
            let mut new_msg = *msg;
            let mut iov = build_iovec(redacted);
            new_msg.msg_iov = &mut iov;
            new_msg.msg_iovlen = 1;
            real!(sendmsg)(sockfd, &new_msg, flags)
        }).unwrap_or_else(|| real!(sendmsg)(sockfd, msg, flags))
    }
}
