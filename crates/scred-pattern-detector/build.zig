const std = @import("std");

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});

    // Zig library for pattern detection
    const lib = b.addStaticLibrary(.{
        .name = "scred_pattern_detector",
        .root_source_file = b.path("src/lib.zig"),
        .target = target,
        .optimize = optimize,
    });

    // Export as C library (for FFI)
    lib.linkLibC();

    b.installArtifact(lib);

    // Unit tests
    const unit_tests = b.addTest(.{
        .root_source_file = b.path("src/lib.zig"),
        .target = target,
        .optimize = optimize,
    });

    const run_unit_tests = b.addRunArtifact(unit_tests);

    // Comprehensive test suite (lib.zig + tests.zig)
    const comprehensive_tests = b.addTest(.{
        .root_source_file = b.path("src/lib.zig"),
        .target = target,
        .optimize = optimize,
    });

    const run_comprehensive_tests = b.addRunArtifact(comprehensive_tests);

    const test_step = b.step("test", "Run unit tests");
    test_step.dependOn(&run_unit_tests.step);

    const fuzz_step = b.step("fuzz", "Run fuzzing and edge case tests");
    fuzz_step.dependOn(&run_comprehensive_tests.step);

    // Benchmarking binary
    const bench = b.addExecutable(.{
        .name = "benchmark",
        .root_source_file = b.path("src/benchmark.zig"),
        .target = target,
        .optimize = optimize,
    });

    const run_bench = b.addRunArtifact(bench);
    const bench_step = b.step("bench", "Run benchmark");
    bench_step.dependOn(&run_bench.step);
}
