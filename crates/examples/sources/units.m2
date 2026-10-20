// units: Initialize fields in declaration order; pub allows access outside methods.
// Run: mlc run units.m2
// Expected output (/ separates lines): point=(30,20)
import std;
using println=std::println;

unit Point {
    pub x:i32;
    pub y:i32;
};

func main() -> i32 {
    var point = Point { 10, 20 };
    point.x = 30;
    println!("point=({},{})", point.x, point.y);
    return 0;
}
