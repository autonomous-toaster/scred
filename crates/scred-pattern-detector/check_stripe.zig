const std = @import("std");

pub fn main() !void {
    // Check the patterns
    const input = "sk_live_test1234567890";
    std.debug.print("Input: {s} (len={})\n", .{ input, input.len });
    
    const stripe_live = "sk_live_";
    const stripe_test = "sk_test_";
    
    const has_live = std.mem.startsWith(u8, input, stripe_live);
    const has_test = std.mem.startsWith(u8, input, stripe_test);
    
    std.debug.print("Has stripe_live prefix: {}\n", .{has_live});
    std.debug.print("Has stripe_test prefix: {}\n", .{has_test});
    std.debug.print("Min len required: 32, actual: {}\n", .{input.len});
}
