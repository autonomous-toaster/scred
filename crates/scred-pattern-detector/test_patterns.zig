const std = @import("std");

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    
    const patterns = [_]struct { prefix: []const u8, input: []const u8 }{
        .{ .prefix = "sk_live_", .input = "sk_live_test1234567890" },
        .{ .prefix = "sk-proj-", .input = "sk-proj-super-long-key-here" },
        .{ .prefix = "postgres://", .input = "postgres://user:pass@host/db" },
    };
    
    for (patterns) |p| {
        const matches = std.mem.startsWith(u8, p.input, p.prefix);
        std.debug.print("'{s}' in '{s}': {}\n", .{ p.prefix, p.input, matches });
    }
}
