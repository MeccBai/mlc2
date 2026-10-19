// generics: Infer T from arguments or write swap<i32>; @mut borrows mutably and $ dereferences.
// Run: mlc run generics.m2
// Expected output (/ separates lines): a=20 b=10
import c_std::io;

func<T> swap(a:$mut T, b:$mut T) {
    var temporary = $a;
    $a = $b;
    $b = temporary;
}

func main() -> i32 {
    var a = 10;
    var b = 20;
    swap(@mut a, @mut b); // Infer T=i32 from the references; swap<i32>(...) also works.
    c_std::io::printf(const_c_str("a=%d b=%d\n"), a, b);
    return 0;
}
