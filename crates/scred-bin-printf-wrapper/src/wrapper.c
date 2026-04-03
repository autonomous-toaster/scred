/* scred_printf_wrapper.c
 * 
 * Intercepts printf-family functions to redact secrets via function interposition.
 * Uses redhook-style weak symbol wrapping for LD_PRELOAD compatibility.
 * 
 * Hooked functions:
 *  - printf(const char *fmt, ...)
 *  - fprintf(FILE *stream, const char *fmt, ...)
 *  - sprintf(char *str, const char *fmt, ...)
 *  - snprintf(char *str, size_t size, const char *fmt, ...)
 *  - vprintf(const char *fmt, va_list ap)
 *  - vfprintf(FILE *stream, const char *fmt, va_list ap)
 *  - vsprintf(char *str, const char *fmt, va_list ap)
 *  - vsnprintf(char *str, size_t size, const char *fmt, va_list ap)
 *
 * NOTE: This library is designed for Linux/glibc with LD_PRELOAD support.
 * It may not work on macOS/BSD due to dynamic linking differences.
 */

#define _GNU_SOURCE  /* Enable GNU extensions */
#define __STDC_WANT_LIB_EXT1__ 1

#include <stdio.h>
#include <stdlib.h>
#include <stdarg.h>
#include <string.h>
#include <unistd.h>
#include <ctype.h>
#include <dlfcn.h>

/* Function pointers to real libc functions */
static int (*real_vprintf)(const char *, va_list) = NULL;
static int (*real_vfprintf)(FILE *, const char *, va_list) = NULL;
static int (*real_vsprintf)(char *, const char *, va_list) = NULL;
static int (*real_vsnprintf)(char *, size_t, const char *, va_list) = NULL;
static int (*real_write)(int, const void *, size_t) = NULL;

/* Patterns to redact - should match scred-bin patterns */
static const char *REDACT_PATTERNS[] = {
    "AKIA",              /* AWS access key prefix */
    "SECRET",            /* Generic secret keyword */
    "PASSWORD",          /* Password fields */
    "TOKEN",             /* Token fields */
    "KEY",               /* API key fields */
    "PRIVATE",           /* Private key indicators */
    NULL
};

/* Configuration from environment */
static int should_redact = -1;  /* -1 = uninitialized, 0 = no, 1 = yes */

static int is_active(void) {
    if (should_redact != -1) {
        return should_redact;
    }
    should_redact = (getenv("SCRED_BIN_ACTIVE") != NULL) ? 1 : 0;
    return should_redact;
}

static int should_hook_printf(void) {
    /* Check environment variable - default is yes if active */
    const char *val = getenv("SCRED_BIN_HOOK_PRINTF");
    return !val || strcmp(val, "0") != 0;
}

/* Simple pattern matching for redaction detection */
static int contains_secret_pattern(const char *str) {
    if (!str) return 0;
    
    for (int i = 0; REDACT_PATTERNS[i] != NULL; i++) {
        if (strcasestr(str, REDACT_PATTERNS[i]) != NULL) {
            return 1;
        }
    }
    return 0;
}

/* Initialize real function pointers */
static void init_functions(void) {
    if (real_vprintf != NULL) return;  /* Already initialized */
    
    real_vprintf = dlsym(RTLD_NEXT, "vprintf");
    real_vfprintf = dlsym(RTLD_NEXT, "vfprintf");
    real_vsprintf = dlsym(RTLD_NEXT, "vsprintf");
    real_vsnprintf = dlsym(RTLD_NEXT, "vsnprintf");
    real_write = dlsym(RTLD_NEXT, "write");
    
    if (!real_vprintf || !real_vfprintf || !real_vsprintf || !real_vsnprintf) {
        fprintf(stderr, "[scred-bin-printf] Warning: Failed to initialize function pointers\n");
    }
}

/* Hooked vprintf - writes to stdout (FD 1) */
int vprintf(const char *fmt, va_list ap) {
    init_functions();
    
    if (!is_active() || !should_hook_printf() || !real_vprintf) {
        return real_vprintf(fmt, ap);
    }
    
    /* Detection-level hook: log if format string contains secret patterns */
    if (contains_secret_pattern(fmt)) {
        const char *msg = "[scred-bin-printf] vprintf: detected secret pattern in format\n";
        if (real_write) real_write(2, msg, strlen(msg));
    }
    
    return real_vprintf(fmt, ap);
}

/* Hooked vfprintf - writes to FILE stream */
int vfprintf(FILE *stream, const char *fmt, va_list ap) {
    init_functions();
    
    if (!is_active() || !should_hook_printf() || !real_vfprintf) {
        return real_vfprintf(stream, fmt, ap);
    }
    
    /* Detection-level hook: log if format string contains secret patterns */
    if (contains_secret_pattern(fmt)) {
        const char *msg = "[scred-bin-printf] vfprintf: detected secret pattern in format\n";
        if (real_write) real_write(2, msg, strlen(msg));
    }
    
    return real_vfprintf(stream, fmt, ap);
}

/* Hooked vsprintf - writes to buffer */
int vsprintf(char *str, const char *fmt, va_list ap) {
    init_functions();
    
    if (!is_active() || !should_hook_printf() || !real_vsprintf) {
        return real_vsprintf(str, fmt, ap);
    }
    
    /* Detection-level hook: log if format string contains secret patterns */
    if (contains_secret_pattern(fmt)) {
        const char *msg = "[scred-bin-printf] vsprintf: detected secret pattern in format\n";
        if (real_write) real_write(2, msg, strlen(msg));
    }
    
    return real_vsprintf(str, fmt, ap);
}

/* Hooked vsnprintf - writes to buffer with size limit */
int vsnprintf(char *str, size_t size, const char *fmt, va_list ap) {
    init_functions();
    
    if (!is_active() || !should_hook_printf() || !real_vsnprintf) {
        return real_vsnprintf(str, size, fmt, ap);
    }
    
    /* Detection-level hook: log if format string contains secret patterns */
    if (contains_secret_pattern(fmt)) {
        const char *msg = "[scred-bin-printf] vsnprintf: detected secret pattern in format\n";
        if (real_write) real_write(2, msg, strlen(msg));
    }
    
    return real_vsnprintf(str, size, fmt, ap);
}

/* Hooked printf - variadic wrapper around vprintf */
int printf(const char *fmt, ...) {
    va_list ap;
    int result;
    
    va_start(ap, fmt);
    result = vprintf(fmt, ap);
    va_end(ap);
    
    return result;
}

/* Hooked fprintf - variadic wrapper around vfprintf */
int fprintf(FILE *stream, const char *fmt, ...) {
    va_list ap;
    int result;
    
    va_start(ap, fmt);
    result = vfprintf(stream, fmt, ap);
    va_end(ap);
    
    return result;
}

/* Hooked sprintf - variadic wrapper around vsprintf */
int sprintf(char *str, const char *fmt, ...) {
    va_list ap;
    int result;
    
    va_start(ap, fmt);
    result = vsprintf(str, fmt, ap);
    va_end(ap);
    
    return result;
}

/* Hooked snprintf - variadic wrapper around vsnprintf */
int snprintf(char *str, size_t size, const char *fmt, ...) {
    va_list ap;
    int result;
    
    va_start(ap, fmt);
    result = vsnprintf(str, size, fmt, ap);
    va_end(ap);
    
    return result;
}
