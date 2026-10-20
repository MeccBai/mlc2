import std::base;
import std::mem;
export using String=std::base::String;
export using ConstStringView=std::base::ConstStringView;

export generic Display { pub func format(self)->String; };

// Private implementation detail. It never enters the module export table.
unit FormatWriter { output:String; pattern:ConstStringView; offset:u64; };
pub FormatWriter::func new(pattern:str)->FormatWriter { return FormatWriter{String::new(),ConstStringView{const_c_str(pattern),str::len(pattern)},0}; }
pub FormatWriter::func flush(mut self) {
    while(self->offset<self->pattern.length){
        val byte=self->pattern.data[self->offset];
        self->offset=self->offset+cast<u64>(1);
        if(byte==cast<i8>(123)){
            val next=self->pattern.data[self->offset];
            self->offset=self->offset+cast<u64>(1);
            if(next==cast<i8>(125)){return;}
            self->output.push_byte(cast<i8>(123));
        }else{
            if(byte==cast<i8>(125)){self->offset=self->offset+cast<u64>(1);}
            self->output.push_byte(byte);
        }
    }
}
pub FormatWriter::func<T:Display> format(mut self,value:T) {
    self->flush();
    var fragment=value.format();
    self->output.append(fragment.view());
}
pub FormatWriter::func finalize(mut self)->String { self->flush();return self->output.clone(); }

export macro func format!(str:str,args:std::macro::Args<std::fmt::Display>)->String {
    var fmt_writer=FormatWriter::new(str);
    macro for arg in args { fmt_writer.format(arg); }
    return fmt_writer.finalize();
}

#[c_abi]# func snprintf(buffer:$mut i8,capacity:u64,pattern:$i8,...)->i32;
func signed(value:i64)->String {
    var bytes:res=std::mem::alloc<i8>(128);
    val length=snprintf(@mut bytes[0],cast<u64>(128),const_c_str("%lld"),value);
    if(length<0){return String::new();}
    return String::from(ConstStringView{@bytes[0],cast<u64>(length)});
}
func unsigned(value:u64)->String {
    var bytes:res=std::mem::alloc<i8>(128);
    val length=snprintf(@mut bytes[0],cast<u64>(128),const_c_str("%llu"),value);
    if(length<0){return String::new();}
    return String::from(ConstStringView{@bytes[0],cast<u64>(length)});
}
func floating(value:f64)->String {
    var bytes:res=std::mem::alloc<i8>(128);
    val length=snprintf(@mut bytes[0],cast<u64>(128),const_c_str("%.17g"),value);
    if(length<0){return String::new();}
    return String::from(ConstStringView{@bytes[0],cast<u64>(length)});
}
api i8::func format(self)->String { return signed(cast<i64>($self)); }
api i16::func format(self)->String { return signed(cast<i64>($self)); }
api i32::func format(self)->String { return signed(cast<i64>($self)); }
api i64::func format(self)->String { return signed(cast<i64>($self)); }
api u8::func format(self)->String { return unsigned(cast<u64>($self)); }
api u16::func format(self)->String { return unsigned(cast<u64>($self)); }
api u32::func format(self)->String { return unsigned(cast<u64>($self)); }
api u64::func format(self)->String { return unsigned(cast<u64>($self)); }
api f32::func format(self)->String { return floating(cast<f64>($self)); }
api f64::func format(self)->String { return floating(cast<f64>($self)); }
api bool::func format(self)->String { if($self){return String::from("true");}return String::from("false"); }
api String::func format(self)->String { return self->clone(); }
api ConstStringView::func format(self)->String { return String::from($self); }

// Write bytes rather than treating formatted output as a C format string.
#[c_abi]# func putchar(value:i32)->i32;
func write(text:ConstStringView) {
    for i in [cast<u64>(0),text.length] { putchar(cast<i32>(text.data[i]) & 255); }
}
export macro func print!(str:str,args:std::macro::Args<std::fmt::Display>) {
    var writer=FormatWriter::new(str);
    macro for arg in args { writer.format(arg); }
    var output=writer.finalize();
    write(output.view());
}
export macro func println!(str:str,args:std::macro::Args<std::fmt::Display>) {
    var writer=FormatWriter::new(str);
    macro for arg in args { writer.format(arg); }
    var output=writer.finalize();
    write(output.view());
    putchar(10);
}


api str::func format(self)->String { return String::from(ConstStringView{const_c_str($self),str::len($self)}); }
