import c_std::lib;
import std::host;
import std::mem;

export unit<T> Vec {
    data: res $mut T;
    length: u64;
    capacity: u64;
};

export Vec<T>::func new() -> Vec<T> {
    const initial_capacity: u64 = 1;
    var data: res = std::mem::alloc<T>(initial_capacity);
    return Vec<T>{data, 0, initial_capacity};
}

api Vec<T>::func len(self) -> u64 {
    return self->length;
}

api Vec<T>::func push(mut self, value: T) {
    if (self->length == self->capacity) {
        const two: u64 = 2;
        if (self->capacity > u64::max() / two) {
            c_std::lib::abort();
        }
        val capacity = self->capacity * two;
        var data: res = std::mem::alloc<T>(capacity);
        var i: u64 = 0;
        while (i < self->length) {
            data[i] = self->data[i];
            i = i + 1;
        }
        self->data = data;
        self->capacity = capacity;
    }
    self->data[self->length] = value;
    self->length = self->length + 1;
}

func check_index(index: u64, length: u64) {
    if (index >= length) {
        c_std::lib::abort();
    }
}

api Vec<T>::func get(self, index: u64) -> T {
    if (std::host::is_debug()) {
        check_index(index, self->length);
    }
    return self->data[index];
}

api Vec<T>::func set(mut self, index: u64, value: T) {
    if (std::host::is_debug()) {
        check_index(index, self->length);
    }
    self->data[index] = value;
}

export unit<T> VecIter {
    data: $T;
    index: u64;
    length: u64;
};

export unit<T> VecIterMut {
    data: $mut T;
    index: u64;
    length: u64;
};

api Vec<T>::func iter(self) -> VecIter<T> {
    return VecIter<T>{@self->data[0], 0, self->length};
}

api Vec<T>::func iter_mut(mut self) -> VecIterMut<T> {
    return VecIterMut<T>{@mut self->data[0], 0, self->length};
}

api VecIter<T>::func has_next(self) -> bool {
    return self->index < self->length;
}

api VecIter<T>::func to_next(mut self) {
    self->index = self->index + 1;
}

api VecIter<T>::func get_ref(self) -> $T {
    if (std::host::is_debug()) {
        check_index(self->index, self->length);
    }
    return @self->data[self->index];
}

api VecIter<T>::func get_clone(self) -> T {
    if (std::host::is_debug()) {
        check_index(self->index, self->length);
    }
    return self->data[self->index];
}

api VecIterMut<T>::func has_next(self) -> bool {
    return self->index < self->length;
}

api VecIterMut<T>::func to_next(mut self) {
    self->index = self->index + 1;
}

api VecIterMut<T>::func get_ref(self) -> $mut T {
    if (std::host::is_debug()) {
        check_index(self->index, self->length);
    }
    return @mut self->data[self->index];
}

api VecIterMut<T>::func get_clone(self) -> T {
    if (std::host::is_debug()) {
        check_index(self->index, self->length);
    }
    return self->data[self->index];
}
