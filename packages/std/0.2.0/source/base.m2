import c_std::lib;
import std::mem;





// Non-owning UTF-8 bytes. String literals convert to this builtin view implicitly.
// length is measured in bytes and excludes the trailing NUL.
export unit ConstStringView {
    pub data: $i8;
    pub length: u64;
};

api ConstStringView::func new(value: ConstStringView) -> ConstStringView {
    return value;
}


api ConstStringView::func len(self) -> u64 {
    return self -> length;
}


api ConstStringView::func is_empty(self) -> bool {
    return self -> length == cast<u64>(0);
}


// Owned, growable, NUL-terminated UTF-8 bytes. Resource storage is cleaned up automatically.
export unit String {
    data: res $mut i8;
    length: u64;
    capacity: u64;
};

api String::func new() -> String {
    var data: res = std::mem::alloc<i8>(1);
    data[0] = 0;
    return String {data, 0, 1};
}


// Public capacity counts usable bytes, excluding the trailing NUL.
api String::func with_capacity(capacity: u64) -> String {
    if (capacity == cast<u64>(18446744073709551615)) {
        c_std::lib::abort();
    }
    val allocated = capacity + cast<u64>(1);
    var data: res = std::mem::alloc<i8>(allocated);
    data[0] = 0;
    return String {data, 0, allocated};
}


api String::func empty() -> String {
    return String::new();
}


api String::func len(self) -> u64 {
    return self -> length;
}


api String::func is_empty(self) -> bool {
    return self -> length == cast<u64>(0);
}


api String::func view(self) -> ConstStringView {
    return ConstStringView {@self -> data[0], self -> length};
}


api String::func as_ptr(self) -> $i8 {
    return @self -> data[0];
}


api String::func reserve(mut self, required: u64) {
    if (required <= self -> capacity) {
        return;
    }
    var capacity = self -> capacity;
    while (capacity < required) {
        if (capacity > cast<u64>(9223372036854775807)) {
            capacity = required;
        } else {
            capacity = capacity * cast<u64>(2);
        }
    }
    var fresh: res = std::mem::alloc<i8>(capacity);
    var i: u64 = 0;
    while (i < self -> length) {
        fresh[i] = self -> data[i];
        i = i + cast<u64>(1);
    }
    fresh[self -> length] = 0;
    self -> data = fresh;
    self -> capacity = capacity;
}


api String::func append(mut self, value: ConstStringView) {
    val maximum: u64 = 18446744073709551615;
    if (value.length >= maximum - self -> length) {
        c_std::lib::abort();
    }
    val required = self -> length + value.length + cast<u64>(1);
    if (required > self -> capacity) {
        var capacity = self -> capacity;
        while (capacity < required) {
            if (capacity > cast<u64>(9223372036854775807)) {
                capacity = required;
            } else {
                capacity = capacity * cast<u64>(2);
            }
        }
        var fresh: res = std::mem::alloc<i8>(capacity);
        var old: u64 = 0;
        while (old < self -> length) {
            fresh[old] = self -> data[old];
            old = old + cast<u64>(1);
        }
        var extra: u64 = 0;
        while (extra < value.length) {
            fresh[self -> length + extra] = value.data[extra];
            extra = extra + cast<u64>(1);
        }
        fresh[required - cast<u64>(1)] = 0;
        // Copy the input first: it may be a view into our existing allocation.
        self -> data = fresh;
        self -> capacity = capacity;
    } else {
        var i: u64 = 0;
        while (i < value.length) {
            self -> data[self -> length + i] = value.data[i];
            i = i + cast<u64>(1);
        }
    }
    self -> length = required - cast<u64>(1);
    self -> data[self -> length] = 0;
}


api String::func push_byte(mut self, value: i8) {
    if (self -> length > cast<u64>(18446744073709551613)) {
        c_std::lib::abort();
    }
    self -> reserve(self -> length + cast<u64>(2));
    self -> data[self -> length] = value;
    self -> length = self -> length + cast<u64>(1);
    self -> data[self -> length] = 0;
}


api String::func from(value: ConstStringView) -> String {
    if (value.length == cast<u64>(18446744073709551615)) {
        c_std::lib::abort();
    }
    val capacity = value.length + cast<u64>(1);
    var data: res = std::mem::alloc<i8>(capacity);
    var i: u64 = 0;
    while (i < value.length) {
        data[i] = value.data[i];
        i = i + cast<u64>(1);
    }
    data[value.length] = 0;
    return String {data, value.length, capacity};
}


api String::func clone(self) -> String {
    return String::from(self -> view());
}
