define void @Default_Handler() noreturn {
entry:
  br label %halt
halt:
  br label %halt
}

define void @Reset_Handler() noreturn {
entry:
  br label %copy
copy:
  %src = phi ptr [ @_sidata, %entry ], [ %next_src, %copy_byte ]
  %dst = phi ptr [ @_sdata, %entry ], [ %next_dst, %copy_byte ]
  %copy_done = icmp uge ptr %dst, @_edata
  br i1 %copy_done, label %zero, label %copy_byte
copy_byte:
  %value = load volatile i8, ptr %src, align 1
  store volatile i8 %value, ptr %dst, align 1
  %next_src = getelementptr i8, ptr %src, i32 1
  %next_dst = getelementptr i8, ptr %dst, i32 1
  br label %copy
zero:
  %bss = phi ptr [ @_sbss, %copy ], [ %next_bss, %zero_byte ]
  %zero_done = icmp uge ptr %bss, @_ebss
  br i1 %zero_done, label %start, label %zero_byte
zero_byte:
  store volatile i8 0, ptr %bss, align 1
  %next_bss = getelementptr i8, ptr %bss, i32 1
  br label %zero
start:
