// formatting: Build a String, escape braces, and control trailing newlines.
// Run: mlc run formatting.m2
// Expected output (/ separates lines): answer=42 ready=true / {value}=1.5 / done / (empty line)
import std;

using format=std::format;
using print=std::print;
using println=std::println;

func main()->i32 {
    var message=format!("answer={} ready={}",42,true);
    println!("{}",message.view());
    println!("{{value}}={}",cast<f32>(1.5));
    print!("do"); // No trailing newline.
    println!("ne");
    println!();
    // Only {} placeholders are supported; each must have exactly one argument.
    return 0;
}
