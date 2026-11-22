
unit TestRange { pub current: i32; pub end: i32; };
func test_range(start: i32, end: i32) -> TestRange {
    return TestRange{start, end};
}
pub TestRange::func has_next(self) -> bool { return self->current < self->end; }
pub TestRange::func to_next(mut self) { self->current = self->current + 1; }
pub TestRange::func get_ref(self) -> $i32 { return @self->current; }
pub TestRange::func get_clone(self) -> i32 { return self->current; }
