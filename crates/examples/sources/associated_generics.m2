// associated_generics: Base is an ordinary unit; the type parameter T belongs to swap.
// Run: mlc run associated_generics.m2
// Expected output: a=100 b=10 fa=80 fb=20
import std;
using println=std::println;
unit Base {};
pub Base::func<T> swap(a:$mut T,b:$mut T) {
    val temporary=$a;
    $a=$b;
    $b=temporary;
}
func main()->i32 {
    var a=10;
    var b=100;
    var fa=20.0;
    var fb=80.0;
    Base::swap(@mut a,@mut b); // Infer T=i32 from the reference arguments.
    Base::swap<f64>(@mut fa,@mut fb); // Explicit type arguments follow the function name.
    // Base::swap(@mut fa,@mut fb) also works, inferring f64.
    println!("a={} b={} fa={} fb={}",a,b,fa,fb);
    return 0;
}
