

# Hard rules

- Patterns belongs to the zig world
- Regexes must be analyzed to asses if they can be decomposed in prefix + lenght or prefix + leght + validation to take advantage of SIMD
- no regex or pattern matching in rust. period.
- patterns must be checked for overlap or duplicates.
