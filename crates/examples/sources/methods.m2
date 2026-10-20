// methods: Keep value private and access it through public methods.
// Run: mlc run methods.m2
// Expected output: value=42
import std;
using println=std::println;
unit Counter { value:i32; };
pub Counter::func read(self)->i32 { return self->value; } // self is a read-only reference.
pub Counter::func set(mut self,value:i32) { self->value=value; }
func main()->i32 {
    var counter=Counter{0};
    counter.set(21);             // Use a dot for an instance.
    val borrowed=@mut counter;
    borrowed->set(42);           // Use an arrow for a reference.
    println!("value={}",counter.read());
    return 0;
}
