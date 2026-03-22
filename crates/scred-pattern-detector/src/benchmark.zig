const std = @import("std");
const lib = @import("lib.zig");

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();

    std.debug.print("\n╔════════════════════════════════════════════════╗\n", .{});
    std.debug.print("║   SCRED Pattern Detector - Throughput Benchmark   ║\n", .{});
    std.debug.print("╚════════════════════════════════════════════════╝\n\n", .{});

    // Benchmark 1: No patterns (baseline)
    try benchmarkNoPatterns(allocator);

    // Benchmark 2: Single match per chunk
    try benchmarkSingleMatch(allocator);

    // Benchmark 3: Multiple matches
    try benchmarkMultipleMatches(allocator);

    // Benchmark 4: Large file simulation
    try benchmarkLargeFile(allocator);

    // Benchmark 5: Real-world patterns mix
    try benchmarkRealWorldMix(allocator);

    std.debug.print("\n✅ Benchmarks complete!\n\n", .{});
}

fn benchmarkNoPatterns(allocator: std.mem.Allocator) !void {
    std.debug.print("📊 Benchmark 1: No Patterns (Baseline)\n", .{});
    std.debug.print("{'─':<44}\n", .{""});

    var detector = lib.PatternDetector{
        .allocator = allocator,
        .events = std.ArrayList(lib.DetectionEvent).init(allocator),
        .output = std.ArrayList(u8).init(allocator),
    };
    defer detector.events.deinit(allocator);
    defer detector.output.deinit(allocator);

    // Generate 10MB of random non-matching data
    const data_size: usize = 10_000_000;
    const data = try allocator.alloc(u8, data_size);
    defer allocator.free(data);

    // Fill with safe chars that won't match
    for (data) |*byte| {
        byte.* = @intCast((byte.* % 26) + 97); // a-z
    }

    // Warm up
    _ = lib.scred_detector_process(&detector, data.ptr, 1024, false);

    // Benchmark: Process in 1MB chunks
    const timer = try std.time.Timer.start();
    var bytes_processed: usize = 0;
    var chunk_idx: usize = 0;

    while (bytes_processed < data_size) {
        const chunk_size = @min(1_000_000, data_size - bytes_processed);
        const is_eof = (bytes_processed + chunk_size >= data_size);

        _ = lib.scred_detector_process(
            &detector,
            data.ptr + bytes_processed,
            chunk_size,
            is_eof,
        );

        bytes_processed += chunk_size;
        chunk_idx += 1;
    }

    const elapsed = timer.read();
    const elapsed_ms = @as(f64, @floatFromInt(elapsed)) / 1_000_000;
    const throughput_mbs = (data_size / 1_000_000.0) / (elapsed_ms / 1000.0);

    std.debug.print("  Data size:     {d:>10} MB\n", .{data_size / 1_000_000});
    std.debug.print("  Chunks:        {d:>10}\n", .{chunk_idx});
    std.debug.print("  Time:          {d:>10.2} ms\n", .{elapsed_ms});
    std.debug.print("  Throughput:    {d:>10.1} MB/s\n", .{throughput_mbs});
    std.debug.print("  Matches:       {d:>10}\n\n", .{detector.events.items.len});
}

fn benchmarkSingleMatch(allocator: std.mem.Allocator) !void {
    std.debug.print("📊 Benchmark 2: Single Match Per Chunk\n", .{});
    std.debug.print("{'─':<44}\n", .{""});

    var detector = lib.PatternDetector{
        .allocator = allocator,
        .events = std.ArrayList(lib.DetectionEvent).init(allocator),
        .output = std.ArrayList(u8).init(allocator),
    };
    defer detector.events.deinit(allocator);
    defer detector.output.deinit(allocator);

    // Create data with AWS keys embedded
    const chunk_size: usize = 65536; // 64KB chunks
    const chunk_count = 160; // 10MB total
    const data = try allocator.alloc(u8, chunk_size);
    defer allocator.free(data);

    // Fill with pattern + padding
    const aws_key = "AKIAIOSFODNN7EXAMPLE";
    @memcpy(data[0..aws_key.len], aws_key);
    for (data[aws_key.len..]) |*byte| {
        byte.* = 'x';
    }

    // Warm up
    _ = lib.scred_detector_process(&detector, data.ptr, 1024, false);

    // Benchmark
    const timer = try std.time.Timer.start();
    var bytes_processed: usize = 0;

    for (0..chunk_count) |i| {
        const is_eof = (i == chunk_count - 1);
        _ = lib.scred_detector_process(&detector, data.ptr, chunk_size, is_eof);
        bytes_processed += chunk_size;
    }

    const elapsed = timer.read();
    const elapsed_ms = @as(f64, @floatFromInt(elapsed)) / 1_000_000;
    const total_mb = bytes_processed / 1_000_000.0;
    const throughput_mbs = total_mb / (elapsed_ms / 1000.0);

    std.debug.print("  Data size:     {d:>10.1} MB\n", .{total_mb});
    std.debug.print("  Chunk size:    {d:>10} KB\n", .{chunk_size / 1024});
    std.debug.print("  Chunks:        {d:>10}\n", .{chunk_count});
    std.debug.print("  Time:          {d:>10.2} ms\n", .{elapsed_ms});
    std.debug.print("  Throughput:    {d:>10.1} MB/s\n", .{throughput_mbs});
    std.debug.print("  Matches:       {d:>10}\n\n", .{detector.events.items.len});
}

fn benchmarkMultipleMatches(allocator: std.mem.Allocator) !void {
    std.debug.print("📊 Benchmark 3: Multiple Matches Per Chunk\n", .{});
    std.debug.print("{'─':<44}\n", .{""});

    var detector = lib.PatternDetector{
        .allocator = allocator,
        .events = std.ArrayList(lib.DetectionEvent).init(allocator),
        .output = std.ArrayList(u8).init(allocator),
    };
    defer detector.events.deinit(allocator);
    defer detector.output.deinit(allocator);

    // Create data with multiple patterns
    const chunk_size: usize = 65536;
    const chunk_count = 160;
    const data = try allocator.alloc(u8, chunk_size);
    defer allocator.free(data);

    // Embed 5 AWS keys per chunk
    const aws_key = "AKIAIOSFODNN7EXAMPLE";
    const github_key = "ghp_abcdefghijklmnopqrstuvwxyz0123456789";

    for (0..5) |i| {
        const offset = i * 10000;
        if (offset + aws_key.len < chunk_size) {
            @memcpy(data[offset .. offset + aws_key.len], aws_key);
        }
        const offset2 = offset + 5000;
        if (offset2 + github_key.len < chunk_size) {
            @memcpy(data[offset2 .. offset2 + github_key.len], github_key);
        }
    }

    // Warm up
    _ = lib.scred_detector_process(&detector, data.ptr, 1024, false);

    // Benchmark
    const timer = try std.time.Timer.start();
    var bytes_processed: usize = 0;

    for (0..chunk_count) |i| {
        const is_eof = (i == chunk_count - 1);
        _ = lib.scred_detector_process(&detector, data.ptr, chunk_size, is_eof);
        bytes_processed += chunk_size;
    }

    const elapsed = timer.read();
    const elapsed_ms = @as(f64, @floatFromInt(elapsed)) / 1_000_000;
    const total_mb = bytes_processed / 1_000_000.0;
    const throughput_mbs = total_mb / (elapsed_ms / 1000.0);

    std.debug.print("  Data size:     {d:>10.1} MB\n", .{total_mb});
    std.debug.print("  Chunk size:    {d:>10} KB\n", .{chunk_size / 1024});
    std.debug.print("  Chunks:        {d:>10}\n", .{chunk_count});
    std.debug.print("  Time:          {d:>10.2} ms\n", .{elapsed_ms});
    std.debug.print("  Throughput:    {d:>10.1} MB/s\n", .{throughput_mbs});
    std.debug.print("  Matches:       {d:>10}\n", .{detector.events.items.len});
    std.debug.print("  Avg matches/chunk: {d:>7.1}\n\n", 
        .{@as(f64, @floatFromInt(detector.events.items.len)) / @as(f64, @floatFromInt(chunk_count))});
}

fn benchmarkLargeFile(allocator: std.mem.Allocator) !void {
    std.debug.print("📊 Benchmark 4: Large File Simulation (100MB)\n", .{});
    std.debug.print("{'─':<44}\n", .{""});

    var detector = lib.PatternDetector{
        .allocator = allocator,
        .events = std.ArrayList(lib.DetectionEvent).init(allocator),
        .output = std.ArrayList(u8).init(allocator),
    };
    defer detector.events.deinit(allocator);
    defer detector.output.deinit(allocator);

    const total_size: usize = 100_000_000; // 100MB
    const chunk_size: usize = 4_000_000; // 4MB chunks
    const chunk_count = total_size / chunk_size;

    // Generate chunk with mixed patterns
    const chunk_data = try allocator.alloc(u8, chunk_size);
    defer allocator.free(chunk_data);

    // Fill with patterns at regular intervals
    var pattern_idx: usize = 0;
    const patterns = [_][]const u8{
        "AKIAIOSFODNN7EXAMPLE",
        "ghp_abcdefghijklmnopqrstuvwxyz0123456789",
        "sk_live_1234567890abcdefghij",
        "sk-proj-1234567890abcdefghijk",
        "postgres://user:pass@host/db",
    };

    for (0..chunk_size / 10000) |i| {
        const offset = i * 10000;
        const pattern = patterns[pattern_idx % patterns.len];
        if (offset + pattern.len < chunk_size) {
            @memcpy(chunk_data[offset .. offset + pattern.len], pattern);
        }
        pattern_idx += 1;
    }

    // Warm up
    _ = lib.scred_detector_process(&detector, chunk_data.ptr, 1024, false);

    // Benchmark
    const timer = try std.time.Timer.start();

    for (0..chunk_count) |i| {
        const is_eof = (i == chunk_count - 1);
        _ = lib.scred_detector_process(&detector, chunk_data.ptr, chunk_size, is_eof);
    }

    const elapsed = timer.read();
    const elapsed_ms = @as(f64, @floatFromInt(elapsed)) / 1_000_000;
    const elapsed_sec = elapsed_ms / 1000.0;
    const total_mb = total_size / 1_000_000.0;
    const throughput_mbs = total_mb / elapsed_sec;

    std.debug.print("  Data size:     {d:>10.1} MB\n", .{total_mb});
    std.debug.print("  Chunk size:    {d:>10.1} MB\n", .{@as(f64, @floatFromInt(chunk_size)) / 1_000_000});
    std.debug.print("  Chunks:        {d:>10}\n", .{chunk_count});
    std.debug.print("  Time:          {d:>10.2} s\n", .{elapsed_sec});
    std.debug.print("  Throughput:    {d:>10.1} MB/s\n", .{throughput_mbs});
    std.debug.print("  Matches:       {d:>10}\n", .{detector.events.items.len});
    std.debug.print("  Per-chunk avg: {d:>10.1} ms\n\n", 
        .{elapsed_ms / @as(f64, @floatFromInt(chunk_count))});
}

fn benchmarkRealWorldMix(allocator: std.mem.Allocator) !void {
    std.debug.print("📊 Benchmark 5: Real-World Mix (Various Patterns)\n", .{});
    std.debug.print("{'─':<44}\n", .{""});

    var detector = lib.PatternDetector{
        .allocator = allocator,
        .events = std.ArrayList(lib.DetectionEvent).init(allocator),
        .output = std.ArrayList(u8).init(allocator),
    };
    defer detector.events.deinit(allocator);
    defer detector.output.deinit(allocator);

    // Create realistic HTTP/2 request payloads
    const http_template = 
        "POST /api/v1/models HTTP/2\r\n"
        "Authorization: Bearer sk-proj-{}\r\n"
        "X-API-Key: {}\r\n"
        "Content-Type: application/json\r\n"
        "\r\n"
        "{{\n"
        "  \"model\": \"gpt-4\",\n"
        "  \"api_key\": \"{}\",\n"
        "  \"messages\": [\n"
        "    {{\"role\": \"user\", \"content\": \"Generate code\"}}\n"
        "  ]\n"
        "}}\n";

    const chunk_size: usize = 65536;
    const chunk_count = 160;
    const chunk_data = try allocator.alloc(u8, chunk_size);
    defer allocator.free(chunk_data);

    // Fill chunk with realistic patterns
    var offset: usize = 0;
    const keys = [_][]const u8{
        "sk-proj-abc123def456",
        "ghp_abcdefghijklmnopqrstuvwxyz01234",
        "AKIAIOSFODNN7EXAMPLE",
        "sk_live_test123456",
    };

    for (0..10) |i| {
        if (offset + 200 < chunk_size) {
            const key = keys[i % keys.len];
            const written = try std.fmt.bufPrint(
                chunk_data[offset..],
                "api_key={s}\n",
                .{key},
            );
            offset += written.len;
        }
    }

    // Warm up
    _ = lib.scred_detector_process(&detector, chunk_data.ptr, 1024, false);

    // Benchmark
    const timer = try std.time.Timer.start();
    var bytes_processed: usize = 0;

    for (0..chunk_count) |i| {
        const is_eof = (i == chunk_count - 1);
        _ = lib.scred_detector_process(&detector, chunk_data.ptr, chunk_size, is_eof);
        bytes_processed += chunk_size;
    }

    const elapsed = timer.read();
    const elapsed_ms = @as(f64, @floatFromInt(elapsed)) / 1_000_000;
    const total_mb = bytes_processed / 1_000_000.0;
    const throughput_mbs = total_mb / (elapsed_ms / 1000.0);

    std.debug.print("  Data size:     {d:>10.1} MB\n", .{total_mb});
    std.debug.print("  Chunk size:    {d:>10} KB\n", .{chunk_size / 1024});
    std.debug.print("  Chunks:        {d:>10}\n", .{chunk_count});
    std.debug.print("  Time:          {d:>10.2} ms\n", .{elapsed_ms});
    std.debug.print("  Throughput:    {d:>10.1} MB/s\n", .{throughput_mbs});
    std.debug.print("  Matches:       {d:>10}\n", .{detector.events.items.len});
    std.debug.print("  Per-chunk avg: {d:>10.3} ms\n\n", 
        .{elapsed_ms / @as(f64, @floatFromInt(chunk_count))});
}
