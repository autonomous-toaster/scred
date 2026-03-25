

# Hard rules
- SIMD is first class citizen in this project
- streaming is first class citizen. no full read.
- Patterns belongs to the zig world
- Regexes must be analyzed to asses if they can be decomposed in prefix + length or prefix + leght + validation to take advantage of SIMD
- pattern than can be simplified from prefix + leght + validation to prefix + length should be to take advatage of SIMD
- no regex or pattern matching in rust. period.
- patterns must be checked for overlap or duplicates.
