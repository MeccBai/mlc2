import c_std::runtime;
import c_std::lib;
import c_std::io;
import std::base;
export using String = std::base::String;
export using ConstStringView = std::base::ConstStringView;


export generic Display {
    pub func format(self) -> String;
};

// Private implementation detail. It never enters the module export table.
unit FormatWriter {
    output: $mut String;
    pattern: ConstStringView;
    offset: u64;
};

pub FormatWriter::func new(pattern: str, output: $mut String) -> FormatWriter {
    return FormatWriter {
        output,
        ConstStringView {
            const_c_str(pattern),
            str::len(pattern)
        },
        0
    };
}


pub FormatWriter::func flush(mut self) {
    var start = self -> offset;
    while (self -> offset < self -> pattern.length) {
        val byte = self -> pattern.data[self -> offset];
        if (byte == cast<i8>(123)) {
            val next = self -> pattern.data[self -> offset + cast<u64>(1)];
            if (next == cast<i8>(125)) {
                if (self -> offset > start) {
                    self -> output -> append(ConstStringView {
                        @self -> pattern.data[start],
                        self -> offset - start
                    });
                }
                self -> offset = self -> offset + cast<u64>(2);
                return;
            }
            self -> output -> append(ConstStringView {
                @self -> pattern.data[start],
                self -> offset - start + cast<u64>(1)
            });
            self -> offset = self -> offset + cast<u64>(2);
            start = self -> offset;
        } else {
            if (byte == cast<i8>(125)) {
                self -> output -> append(ConstStringView {
                    @self -> pattern.data[start],
                    self -> offset - start + cast<u64>(1)
                });
                self -> offset = self -> offset + cast<u64>(2);
                start = self -> offset;
            } else {
                self -> offset = self -> offset + cast<u64>(1);
            }
        }
    }
    if (self -> offset > start) {
        self -> output -> append(ConstStringView {
            @self -> pattern.data[start],
            self -> offset - start
        });
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
    str: str,
    args: std::macro::Args<std::fmt::Display>
) -> String {
    var output = String::with_capacity(str::len(str) + cast<u64>(32));
    anonymous {
        var writer = FormatWriter::new(str, @mut output);
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
    var offset: u64 = 21;
    var remaining = value;
    while (remaining >= cast<u64>(10)) {
        val quotient = remaining / cast<u64>(10);
        offset = offset - cast<u64>(1);
        bytes[offset] = cast<i8>(
            remaining - quotient * cast<u64>(10)
        ) + cast<i8>(48);
        remaining = quotient;
    }
    offset = offset - cast<u64>(1);
    bytes[offset] = cast<i8>(remaining) + cast<i8>(48);
    if (negative) {
        offset = offset - cast<u64>(1);
        bytes[offset] = 45;
    }
    return String::from(ConstStringView {
        @bytes[offset],
        cast<u64>(21) - offset
    });
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
    val negative =(bits >> cast<u64>(63)) != cast<u64>(0);
    val fraction = bits & cast<u64>(4503599627370495);
    val exponent =(bits >> cast<u64>(52)) & cast<u64>(2047);
    if (exponent == cast<u64>(2047)) {
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
    if ((bits & cast<u64>(9223372036854775807)) == cast<u64>(0)) {
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
    while (count < cast<u64>(17) && digits[count] != cast<i8>(0)) {
        count = count + cast<u64>(1);
    }
    if (count == cast<u64>(0)) {
        c_std::lib::abort();
    }
    while (count > cast<u64>(1) && digits[count - cast<u64>(1)] == cast<i8>(48)) {
        count = count - cast<u64>(1);
    }
    var bytes: [i8: 32] = "";
    var length: u64 = 0;
    if (negative) {
        bytes[length] = 45;
        length = length + cast<u64>(1);
    }
    val power = decimal_point - 1;
    if (power < - 4 || power >= 17) {
        bytes[length] = digits[0];
        length = length + cast<u64>(1);
        if (count > cast<u64>(1)) {
            bytes[length] = 46;
            length = length + cast<u64>(1);
            var i: u64 = 1;
            while (i < count) {
                bytes[length] = digits[i];
                length = length + cast<u64>(1);
                i = i + cast<u64>(1);
            }
        }
        bytes[length] = 101;
        length = length + cast<u64>(1);
        var magnitude = power;
        if (power < 0) {
            bytes[length] = 45;
            magnitude = 0 - power;
        } else {
            bytes[length] = 43;
        }
        length = length + cast<u64>(1);
        if (magnitude >= 100) {
            bytes[length] = cast<i8>(magnitude / 100) + cast<i8>(48);
            length = length + cast<u64>(1);
        }
        bytes[length] = cast<i8>((magnitude / 10) % 10) + cast<i8>(48);
        length = length + cast<u64>(1);
        bytes[length] = cast<i8>(magnitude % 10) + cast<i8>(48);
        length = length + cast<u64>(1);
    } else {
        if (decimal_point <= 0) {
            bytes[length] = 48;
            bytes[length + cast<u64>(1)] = 46;
            length = length + cast<u64>(2);
            var zeros = 0 - decimal_point;
            while (zeros > 0) {
                bytes[length] = 48;
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
                    bytes[length] = 46;
                    length = length + cast<u64>(1);
                }
                if (i < count) {
                    bytes[length] = digits[i];
                } else {
                    bytes[length] = 48;
                }
                length = length + cast<u64>(1);
                i = i + cast<u64>(1);
            }
        }
    }
    return String::from(
        ConstStringView {@bytes[0], length}
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


api ConstStringView::func format(self) -> String {
    return String::from($self);
}


// Use the same buffered CRT stream as C stdio, locking once per complete output.
// Explicit byte counts preserve embedded NULs and percent signs.






func write(text: ConstStringView) {
    val stream = c_std::runtime::__acrt_iob_func(cast<u32>(1));
    var offset: u64 = 0;
    while (offset < text.length) {
        val written = c_std::io::fwrite(
            @text.data[offset],
            cast<u64>(1),
            text.length - offset,
            stream
        );
        if (written == cast<u64>(0)) {
            return;
        }
        offset = offset + written;
    }
}


export macro func print!(
    str: str,
    args: std::macro::Args<std::fmt::Display>
) {
    var output = String::with_capacity(str::len(str) + cast<u64>(32));
    anonymous {
        var writer = FormatWriter::new(str, @mut output);
        macro for arg in args {
            writer.format(arg);
        }
        writer.finalize();
    }
    write(output.view());
}


export macro func println!(
    str: str,
    args: std::macro::Args<std::fmt::Display>
) {
    var output = String::with_capacity(str::len(str) + cast<u64>(32));
    anonymous {
        var writer = FormatWriter::new(str, @mut output);
        macro for arg in args {
            writer.format(arg);
        }
        writer.finalize();
    }
    output.push_byte(cast<i8>(10));
    write(output.view());
}


api str::func format(self) -> String {
    return String::from(ConstStringView {
        const_c_str($self),
        str::len($self)
    });
}
