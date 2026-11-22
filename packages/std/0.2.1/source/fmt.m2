import std::bit;
import c_std::runtime;
import c_std::lib;
import c_std::io;
import std::base;
export using String = std::base::String;


export generic Display {
    pub func format(self) -> String;
};

// Private implementation detail. It never enters the module export table.
unit FormatWriter {
    output: $mut String;
    pattern: str;
    offset: u64;
};

pub FormatWriter::func new(pattern: str, output: $mut String) -> FormatWriter {
    return FormatWriter {output, pattern, 0};
}


pub FormatWriter::func flush(mut self) {
    val length = str::len(self -> pattern);
    val data = str::as_ptr(self -> pattern);
    var start = self -> offset;
    while (self -> offset < length) {
        val byte = data[self -> offset];
        if (byte == cast<i8>("{".first_byte())) {
            val next = data[self -> offset + cast<u64>(1)];
            if (next == cast<i8>("}".first_byte())) {
                if (self -> offset > start) {
                    self -> output -> append(str::from_parts(
                        @data[start],
                        self -> offset - start
                    ));
                }
                self -> offset = self -> offset + cast<u64>(2);
                return;
            }
            self -> output -> append(str::from_parts(
                @data[start],
                self -> offset - start + cast<u64>(1)
            ));
            self -> offset = self -> offset + cast<u64>(2);
            start = self -> offset;
        } else {
            if (byte == cast<i8>("}".first_byte())) {
                self -> output -> append(str::from_parts(
                    @data[start],
                    self -> offset - start + cast<u64>(1)
                ));
                self -> offset = self -> offset + cast<u64>(2);
                start = self -> offset;
            } else {
                self -> offset = self -> offset + cast<u64>(1);
            }
        }
    }
    if (self -> offset > start) {
        self -> output -> append(str::from_parts(
            @data[start],
            self -> offset - start
        ));
    }
}


pub FormatWriter::func<T: Display> format(mut self, value: T) {
    self -> flush();
    var fragment = value.format();
    self -> output -> append(fragment.view());
}


pub FormatWriter::func finalize(mut self) {
    self -> flush();
}


export macro func format!(
    pattern: str,
    args: std::macro::Args<std::fmt::Display>
) -> String {
    var output = String::with_capacity(str::len(pattern) + cast<u64>(32));
    anonymous {
        var writer = FormatWriter::new(pattern, @mut output);
        macro for arg in args {
            writer.format(arg);
        }
        writer.finalize();
    }
    return output;
}


// Integer conversion uses a stack buffer and one division per decimal digit.
// The unsigned magnitude also covers i64::MIN without signed negation overflow.
func decimal(value: u64, negative: bool) -> String {
    var bytes: [i8: 21] = "";
    var offset: u64 = 21; // 20 decimal digits plus a possible sign.
    val end = offset;
    var remaining = value;
    while (remaining >= cast<u64>(10)) {
        val quotient = remaining / cast<u64>(10);
        offset = offset - cast<u64>(1);
        bytes[offset] = cast<i8>(
            remaining - quotient * cast<u64>(10)
        ) + cast<i8>("0".first_byte());
        remaining = quotient;
    }
    offset = offset - cast<u64>(1);
    bytes[offset] = cast<i8>(remaining) + cast<i8>("0".first_byte());
    if (negative) {
        offset = offset - cast<u64>(1);
        bytes[offset] = cast<i8>("-".first_byte());
    }
    return String::from(str::from_parts(
        @bytes[offset],
        end - offset
    ));
}


func signed(value: i64) -> String {
    val negative = value < cast<i64>(0);
    var magnitude = cast<u64>(value);
    if (negative) {
        magnitude = cast<u64>(0) - magnitude;
    }
    return decimal(magnitude, negative);
}


func unsigned(value: u64) -> String {
    return decimal(value, false);
}


// The CRT supplies rounded significant digits only. Sign, punctuation, trailing
// zero removal and the general-format exponent policy live in this library.






func floating(value: f64) -> String {
    val representation = cast<$u64>(@value);
    val bits = $representation;
    const fraction_bits: u64 = 52;
    const exponent_bits: u64 = 11;
    const fraction_mask = std::bit::shl(cast<u64>(1), fraction_bits) - cast<u64>(1);
    const exponent_mask = std::bit::shl(cast<u64>(1), exponent_bits) - cast<u64>(1);
    val negative = std::bit::shr(bits, fraction_bits + exponent_bits) != cast<u64>(0);
    val fraction = bits & fraction_mask;
    val exponent = std::bit::shr(bits, fraction_bits) & exponent_mask;
    if (exponent == exponent_mask) {
        if (fraction != cast<u64>(0)) {
            if (negative) {
                return String::from("-nan");
            }
            return String::from("nan");
        }
        if (negative) {
            return String::from("-inf");
        }
        return String::from("inf");
    }
    if ((bits & cast<u64>(i64::max())) == cast<u64>(0)) {
        if (negative) {
            return String::from("-0");
        }
        return String::from("0");
    }
    var digits: [i8: 32] = "";
    var decimal_point: i32 = 0;
    var sign: i32 = 0;
    if (c_std::runtime::_ecvt_s(
        @mut digits[0],
        cast<u64>(32),
        value,
        17,
        @mut decimal_point,
        @mut sign
    ) != 0) {
        c_std::lib::abort();
    }
    var count: u64 = 0;
    while (count < cast<u64>(17) && digits[count] != cast<i8>("\0".first_byte())) {
        count = count + cast<u64>(1);
    }
    if (count == cast<u64>(0)) {
        c_std::lib::abort();
    }
    while (count > cast<u64>(1) && digits[count - cast<u64>(1)] == cast<i8>("0".first_byte())) {
        count = count - cast<u64>(1);
    }
    var bytes: [i8: 32] = "";
    var length: u64 = 0;
    if (negative) {
        bytes[length] = cast<i8>("-".first_byte());
        length = length + cast<u64>(1);
    }
    val power = decimal_point - 1;
    if (power < - 4 || power >= 17) {
        bytes[length] = digits[0];
        length = length + cast<u64>(1);
        if (count > cast<u64>(1)) {
            bytes[length] = cast<i8>(".".first_byte());
            length = length + cast<u64>(1);
            var i: u64 = 1;
            while (i < count) {
                bytes[length] = digits[i];
                length = length + cast<u64>(1);
                i = i + cast<u64>(1);
            }
        }
        bytes[length] = cast<i8>("e".first_byte());
        length = length + cast<u64>(1);
        var magnitude = power;
        if (power < 0) {
            bytes[length] = cast<i8>("-".first_byte());
            magnitude = 0 - power;
        } else {
            bytes[length] = cast<i8>("+".first_byte());
        }
        length = length + cast<u64>(1);
        if (magnitude >= 100) {
            bytes[length] = cast<i8>(magnitude / 100) + cast<i8>("0".first_byte());
            length = length + cast<u64>(1);
        }
        bytes[length] = cast<i8>((magnitude / 10) % 10) + cast<i8>("0".first_byte());
        length = length + cast<u64>(1);
        bytes[length] = cast<i8>(magnitude % 10) + cast<i8>("0".first_byte());
        length = length + cast<u64>(1);
    } else {
        if (decimal_point <= 0) {
            bytes[length] = cast<i8>("0".first_byte());
            bytes[length + cast<u64>(1)] = cast<i8>(".".first_byte());
            length = length + cast<u64>(2);
            var zeros = 0 - decimal_point;
            while (zeros > 0) {
                bytes[length] = cast<i8>("0".first_byte());
                length = length + cast<u64>(1);
                zeros = zeros - 1;
            }
            var i: u64 = 0;
            while (i < count) {
                bytes[length] = digits[i];
                length = length + cast<u64>(1);
                i = i + cast<u64>(1);
            }
        } else {
            var i: u64 = 0;
            val point = cast<u64>(decimal_point);
            while (i < point || i < count) {
                if (i == point) {
                    bytes[length] = cast<i8>(".".first_byte());
                    length = length + cast<u64>(1);
                }
                if (i < count) {
                    bytes[length] = digits[i];
                } else {
                    bytes[length] = cast<i8>("0".first_byte());
                }
                length = length + cast<u64>(1);
                i = i + cast<u64>(1);
            }
        }
    }
    return String::from(
        str::from_parts(@bytes[0], length)
    );
}


api i8::func format(self) -> String {
    return signed(cast<i64>($self));
}


api i16::func format(self) -> String {
    return signed(cast<i64>($self));
}


api i32::func format(self) -> String {
    return signed(cast<i64>($self));
}


api i64::func format(self) -> String {
    return signed(cast<i64>($self));
}


api u8::func format(self) -> String {
    return unsigned(cast<u64>($self));
}


api u16::func format(self) -> String {
    return unsigned(cast<u64>($self));
}


api u32::func format(self) -> String {
    return unsigned(cast<u64>($self));
}


api u64::func format(self) -> String {
    return unsigned(cast<u64>($self));
}


api f32::func format(self) -> String {
    return floating(cast<f64>($self));
}


api f64::func format(self) -> String {
    return floating(cast<f64>($self));
}


api bool::func format(self) -> String {
    if ($self) {
        return String::from("true");
    }
    return String::from("false");
}


api String::func format(self) -> String {
    return self -> clone();
}


// Use the same buffered CRT stream as C stdio, locking once per complete output.
// Explicit byte counts preserve embedded NULs and percent signs.






func write(text: str) {
    val length = str::len(text);
    val data = str::as_ptr(text);
    val stream = c_std::runtime::__acrt_iob_func(cast<u32>(1));
    var offset: u64 = 0;
    while (offset < length) {
        val written = c_std::io::fwrite(
            @data[offset],
            cast<u64>(1),
            length - offset,
            stream
        );
        if (written == cast<u64>(0)) {
            return;
        }
        offset = offset + written;
    }
}


export macro func print!(
    pattern: str,
    args: std::macro::Args<std::fmt::Display>
) {
    var output = String::with_capacity(str::len(pattern) + cast<u64>(32));
    anonymous {
        var writer = FormatWriter::new(pattern, @mut output);
        macro for arg in args {
            writer.format(arg);
        }
        writer.finalize();
    }
    write(output.view());
}


export macro func println!(
    pattern: str,
    args: std::macro::Args<std::fmt::Display>
) {
    var output = String::with_capacity(str::len(pattern) + cast<u64>(32));
    anonymous {
        var writer = FormatWriter::new(pattern, @mut output);
        macro for arg in args {
            writer.format(arg);
        }
        writer.finalize();
    }
    output.push_byte(cast<i8>("\n".first_byte()));
    write(output.view());
}


api str::func format(self) -> String {
    return String::from($self);
}
