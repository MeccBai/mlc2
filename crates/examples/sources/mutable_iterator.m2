// mutable-iterator: A cursor over one borrowed value and writable loop bindings.
// Run: mlc run mutable_iterator.m2
// Expected output: value=42
import std;

unit Cursor { pub slot: $mut i32; pub pending: bool; };
pub Cursor::func has_next(self) -> bool { return self->pending; }
pub Cursor::func to_next(mut self) { self->pending = false; }
pub Cursor::func get_ref(self) -> $mut i32 { return self->slot; }
pub Cursor::func get_clone(self) -> i32 { return $(self->slot); }

func main() -> i32 {
    var number = 41;
    var cursor = Cursor{@mut number, true};
    for $mut value in @mut cursor {
        $value = $value + 1;
    }
    std::println!("value={}", number);
    return 0;
}
