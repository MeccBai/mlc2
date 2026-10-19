// unions: Result holds one active variant; match unpacks it by variant type.
// Run: mlc run unions.m2
// Expected output: value=42
import c_std::io;
unit<T> Success { pub value:T; };
unit Failed { pub code:i32; };
union<T> Result[Success<T>,Failed];
func main()->i32 {
    var result=Result<i32>{Success<i32>{42}};
    match (value=@result) {
        Success<i32> => { c_std::io::printf(const_c_str("value=%d\n"),value->value); },
        Failed => { c_std::io::printf(const_c_str("error=%d\n"),value->code); }
    }
    return 0;
}
