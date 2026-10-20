// strings: Owned UTF-8 storage, literal views, growth, and independent clones.
// Run: mlc run strings.m2
// Expected output (/ separates lines): Hello, world! / bytes=13 empty=false / copy=Hello, world! original=Hello, world!?
import std;
using println=std::println;
func main()->i32 {
    var text=std::String::from("Hello");
    text.append(", world!");
    println!("{}",text.view()); // Pass a view when retaining the owned String.
    println!("bytes={} empty={}",text.len(),text.is_empty());
    var copy=text.clone();
    text.append("?");
    println!("copy={} original={}",copy.view(),text.view());
    return 0;
}
