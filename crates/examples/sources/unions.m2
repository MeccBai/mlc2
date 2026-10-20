// unions: Result holds one active variant; match unpacks it by variant type.
// Run: mlc run unions.m2
// Expected output: value=42
import std;
using println=std::println;
unit<T> Success { pub value:T; };
unit Failed { pub code:i32; };
union<T> Result[Success<T>,Failed];
func main()->i32 {
    var result=Result<i32>{Success<i32>{42}};
    match (value=@result) {
        Success<i32> => { println!("value={}",value->value); },
        Failed => { println!("error={}",value->code); }
    }
    return 0;
}
