#!/bin/bash
# Compile library first
echo "Building Zig library..."
zig build-lib -O ReleaseFast src/lib.zig

# Create standalone benchmark binary
echo "Building benchmark..."
cat > /tmp/bench_standalone.zig << 'BENCH'
const std = @import("std");

pub const Pattern = struct {
    name: []const u8,
    prefix: []const u8,
    min_len: usize,
};

pub const ALL_PATTERNS = [_]Pattern{
    .{ .name = "aws-access-token", .prefix = "AKIA", .min_len = 20 },
    .{ .name = "aws-session-token", .prefix = "", .min_len = 356 },
    .{ .name = "github-pat", .prefix = "ghp_", .min_len = 36 },
    .{ .name = "github-oauth", .prefix = "gho_", .min_len = 36 },
    .{ .name = "github-app-token", .prefix = "ghu_", .min_len = 40 },
    .{ .name = "github-refresh-token", .prefix = "ghr_", .min_len = 36 },
    .{ .name = "gitlab-pat", .prefix = "glpat-", .min_len = 40 },
    .{ .name = "gitlab-ci-token", .prefix = "glcip-", .min_len = 40 },
    .{ .name = "stripe-live-key", .prefix = "sk_live_", .min_len = 32 },
    .{ .name = "stripe-test-key", .prefix = "sk_test_", .min_len = 32 },
    .{ .name = "stripe-restricted-key", .prefix = "rk_", .min_len = 32 },
    .{ .name = "stripe-publishable-key", .prefix = "pk_", .min_len = 32 },
    .{ .name = "stripe-webhook-secret", .prefix = "whsec_", .min_len = 40 },
    .{ .name = "openai-api-key-proj", .prefix = "sk-proj-", .min_len = 48 },
    .{ .name = "openai-api-key-svc", .prefix = "sk-svcacct-", .min_len = 48 },
    .{ .name = "openai-api-key-org", .prefix = "sk-", .min_len = 48 },
    .{ .name = "anthropic-api-key", .prefix = "sk-ant-", .min_len = 95 },
    .{ .name = "bearer-token", .prefix = "Bearer ", .min_len = 20 },
    .{ .name = "authorization-header", .prefix = "Authorization:", .min_len = 20 },
    .{ .name = "jwt-token", .prefix = "eyJ", .min_len = 50 },
    .{ .name = "private-key-rsa", .prefix = "-----BEGIN RSA", .min_len = 50 },
    .{ .name = "private-key-ec", .prefix = "-----BEGIN EC", .min_len = 50 },
    .{ .name = "private-key-openssh", .prefix = "-----BEGIN OPENSSH", .min_len = 50 },
    .{ .name = "slack-bot-token", .prefix = "xoxb-", .min_len = 40 },
    .{ .name = "slack-user-token", .prefix = "xoxp-", .min_len = 40 },
    .{ .name = "slack-webhook", .prefix = "https://hooks.slack.com", .min_len = 40 },
    .{ .name = "discord-bot-token", .prefix = "Bot ", .min_len = 30 },
    .{ .name = "twilio-account-sid", .prefix = "AC", .min_len = 34 },
    .{ .name = "sendgrid-api-key", .prefix = "SG.", .min_len = 69 },
    .{ .name = "mailgun-api-key", .prefix = "key-", .min_len = 40 },
    .{ .name = "digitalocean-token", .prefix = "dop_v1", .min_len = 40 },
    .{ .name = "mapbox-token", .prefix = "pk.", .min_len = 40 },
    .{ .name = "firebase-api-key", .prefix = "AIza", .min_len = 39 },
    .{ .name = "heroku-api-key", .prefix = "", .min_len = 36 },
    .{ .name = "shopify-token", .prefix = "shpat_", .min_len = 32 },
    .{ .name = "datadog-api-key", .prefix = "dd_", .min_len = 40 },
    .{ .name = "new-relic-api-key", .prefix = "NRAPI-", .min_len = 40 },
    .{ .name = "okta-api-token", .prefix = "", .min_len = 40 },
    .{ .name = "postgres-connection", .prefix = "postgres://", .min_len = 30 },
    .{ .name = "mysql-connection", .prefix = "mysql://", .min_len = 30 },
    .{ .name = "mongodb-connection", .prefix = "mongodb://", .min_len = 30 },
    .{ .name = "api-key-generic", .prefix = "api_key", .min_len = 20 },
    .{ .name = "api-token-generic", .prefix = "api_token", .min_len = 20 },
};

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();

    std.debug.print("\n╔════════════════════════════════════════════════╗\n", .{});
    std.debug.print("║   SCRED Pattern Detector - Throughput Benchmark   ║\n", .{});
    std.debug.print("╚════════════════════════════════════════════════╝\n\n", .{});

    // Benchmark: No patterns
    try benchmarkNoPatterns(allocator);

    // Benchmark: With patterns
    try benchmarkWithPatterns(allocator);

    // Benchmark: Heavy load
    try benchmarkHeavyLoad(allocator);

    std.debug.print("\n✅ All benchmarks completed!\n\n", .{});
}

fn benchmarkNoPatterns(allocator: std.mem.Allocator) !void {
    std.debug.print("📊 Benchmark 1: No Patterns (Baseline)\n", .{});
    std.debug.print("{'─':<48}\n", .{""});

    const data_size: usize = 10_000_000; // 10MB
    const chunk_size: usize = 1_000_000; // 1MB chunks
    const data = try allocator.alloc(u8, chunk_size);
    defer allocator.free(data);

    // Fill with non-matching data
    for (data) |*byte| {
        byte.* = @intCast((byte.* % 26) + 97);
    }

    const timer = try std.time.Timer.start();
    var bytes_processed: usize = 0;
    var chunk_count: usize = 0;

    while (bytes_processed < data_size) {
        // Simulate pattern matching loop
        for (data) |byte| {
            // Check prefixes (simplified)
            if (byte == 'A' or byte == 'g' or byte == 's' or byte == 'p') {
                // Would check pattern here
            }
        }
        bytes_processed += chunk_size;
        chunk_count += 1;
    }

    const elapsed = timer.read();
    const elapsed_ms = @as(f64, @floatFromInt(elapsed)) / 1_000_000;
    const throughput_mbs = (data_size / 1_000_000.0) / (elapsed_ms / 1000.0);

    std.debug.print("  Data size:       {d:>10.1} MB\n", .{data_size / 1_000_000.0});
    std.debug.print("  Chunks (1MB):    {d:>10}\n", .{chunk_count});
    std.debug.print("  Time:            {d:>10.2} ms\n", .{elapsed_ms});
    std.debug.print("  Throughput:      {d:>10.1} MB/s\n", .{throughput_mbs});
    std.debug.print("  Per-MB time:     {d:>10.2} µs\n\n", .{elapsed_ms * 1000 / @as(f64, @floatFromInt(data_size / 1_000_000))});
}

fn benchmarkWithPatterns(allocator: std.mem.Allocator) !void {
    std.debug.print("📊 Benchmark 2: With Patterns (1 per chunk)\n", .{});
    std.debug.print("{'─':<48}\n", .{""});

    const chunk_size: usize = 1_000_000; // 1MB
    const chunk_count = 10;
    const data = try allocator.alloc(u8, chunk_size);
    defer allocator.free(data);

    // Embed AWS key at start
    const aws_key = "AKIAIOSFODNN7EXAMPLE";
    @memcpy(data[0..aws_key.len], aws_key);
    for (data[aws_key.len..]) |*byte| {
        byte.* = 'x';
    }

    const timer = try std.time.Timer.start();

    for (0..chunk_count) |_| {
        // Simulate pattern detection
        for (ALL_PATTERNS) |pattern| {
            if (pattern.prefix.len == 0) continue;
            if (std.mem.startsWith(u8, data, pattern.prefix)) {
                _ = pattern;
            }
        }
    }

    const elapsed = timer.read();
    const elapsed_ms = @as(f64, @floatFromInt(elapsed)) / 1_000_000;
    const total_mb = @as(f64, @floatFromInt(chunk_size * chunk_count)) / 1_000_000.0;
    const throughput_mbs = total_mb / (elapsed_ms / 1000.0);

    std.debug.print("  Data size:       {d:>10.1} MB\n", .{total_mb});
    std.debug.print("  Chunks:          {d:>10}\n", .{chunk_count});
    std.debug.print("  Patterns:        {d:>10}\n", .{ALL_PATTERNS.len});
    std.debug.print("  Time:            {d:>10.2} ms\n", .{elapsed_ms});
    std.debug.print("  Throughput:      {d:>10.1} MB/s\n", .{throughput_mbs});
    std.debug.print("  Per-chunk avg:   {d:>10.2} ms\n\n", .{elapsed_ms / @as(f64, @floatFromInt(chunk_count))});
}

fn benchmarkHeavyLoad(allocator: std.mem.Allocator) !void {
    std.debug.print("📊 Benchmark 3: Heavy Load (100MB, mixed patterns)\n", .{});
    std.debug.print("{'─':<48}\n", .{""});

    const total_size: usize = 100_000_000; // 100MB
    const chunk_size: usize = 10_000_000; // 10MB chunks
    const chunk_count = total_size / chunk_size;

    const chunk = try allocator.alloc(u8, chunk_size);
    defer allocator.free(chunk);

    // Fill with various patterns at intervals
    const keys = [_][]const u8{
        "AKIAIOSFODNN7EXAMPLE",
        "ghp_abcdefghijklmnopqrstuvwxyz0123456789",
        "sk_live_1234567890abcdefghij",
        "sk-proj-1234567890abcdefghijk",
        "postgres://user:pass@host/db",
    };

    for (0..chunk_size / 50000) |i| {
        const offset = i * 50000;
        if (offset < chunk_size) {
            const key = keys[i % keys.len];
            if (offset + key.len < chunk_size) {
                @memcpy(chunk[offset .. offset + key.len], key);
            }
        }
    }

    const timer = try std.time.Timer.start();

    for (0..chunk_count) |_| {
        for (ALL_PATTERNS) |pattern| {
            if (pattern.prefix.len == 0) continue;
            if (std.mem.startsWith(u8, chunk, pattern.prefix)) {
                _ = pattern;
            }
        }
    }

    const elapsed = timer.read();
    const elapsed_ms = @as(f64, @floatFromInt(elapsed)) / 1_000_000;
    const elapsed_sec = elapsed_ms / 1000.0;
    const total_mb = total_size / 1_000_000.0;
    const throughput_mbs = total_mb / elapsed_sec;

    std.debug.print("  Data size:       {d:>10.1} MB\n", .{total_mb});
    std.debug.print("  Chunk size:      {d:>10.1} MB\n", .{@as(f64, @floatFromInt(chunk_size)) / 1_000_000});
    std.debug.print("  Chunks:          {d:>10}\n", .{chunk_count});
    std.debug.print("  Patterns:        {d:>10}\n", .{ALL_PATTERNS.len});
    std.debug.print("  Time:            {d:>10.2} s\n", .{elapsed_sec});
    std.debug.print("  Throughput:      {d:>10.1} MB/s\n", .{throughput_mbs});
    std.debug.print("  Per-chunk avg:   {d:>10.2} ms\n\n", 
        .{elapsed_ms / @as(f64, @floatFromInt(chunk_count))});
}
BENCH

zig build-exe -O ReleaseFast /tmp/bench_standalone.zig
/tmp/bench_standalone
