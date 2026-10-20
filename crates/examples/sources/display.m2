// display: A format(self)->String interface satisfies the Display requirement.
// Run: mlc run display.m2
// Expected output: point=Point(3,4)
import std;
using format=std::format;
using println=std::println;
unit Point { pub x:i32; pub y:i32; };
pub Point::func format(self)->std::String {
    return format!("Point({},{})",self->x,self->y);
}
func main()->i32 {
    println!("point={}",Point{3,4});
    return 0;
}
