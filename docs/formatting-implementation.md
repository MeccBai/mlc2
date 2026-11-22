# Formatting implementation

The private `std::fmt::FormatWriter` borrows the result buffer and appends literal
segments in batches. Finalization fills the existing buffer; it does not clone
an owned String. Each primitive conversion uses stack scratch space and creates
its returned String with a single allocation. Custom Display implementations
continue to use the same `format(self)->String` contract.

Signed and unsigned integer conversion is implemented in standard-library source,
including the minimum signed 64-bit value. Floating-point conversion uses the
Windows CRT `_ecvt_s` only for rounded significant digits; punctuation, trailing
zero removal, sign and scientific notation are handled by the standard library.
It preserves the previous 17-significant-digit general format, including f32
promotion to f64. Negative zero is `-0`; infinities and NaNs use `inf`, `-inf`,
`nan`, and `-nan`. There are no `snprintf` calls in the formatting library.

`print!` and `println!` assemble one result and write it through buffered CRT
`fwrite`, sharing stdout with C stdio. A newline is appended before the write.
Partial writes are retried until completion or until the stream makes no progress.
Explicit byte counts preserve embedded NUL bytes and percent signs. No per-byte
`putchar` calls or variadic format-string dispatch are used for these macros.
