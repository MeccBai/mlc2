export generic Iterator<T> {
    func has_next(self) -> bool;
    func to_next(mut self);
    func get_ref(self) -> $T;
    func get_clone(self) -> T;
};

export generic IteratorMut<T> {
    func has_next(self) -> bool;
    func to_next(mut self);
    func get_ref(self) -> $mut T;
    func get_clone(self) -> T;
};

export unit RangeIter {
    current: u64;
    end: u64;
    step: u64;
};

export func range(start: u64, end: u64) -> RangeIter {
    return range_step(start, end, cast<u64>(1));
}

export func range_step(start: u64, end: u64, step: u64) -> RangeIter {
    return RangeIter {start, end, step};
}

api RangeIter::func has_next(self) -> bool {
    return self->step != cast<u64>(0) && self->current < self->end;
}

api RangeIter::func to_next(mut self) {
    if (self->current >= self->end || self->step >= self->end - self->current) {
        self->current = self->end;
    } else {
        self->current = self->current + self->step;
    }
}

api RangeIter::func get_ref(self) -> $u64 {
    return @self->current;
}

api RangeIter::func get_clone(self) -> u64 {
    return self->current;
}
