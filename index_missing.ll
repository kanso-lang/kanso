%KValue = type { i64, i64 }
%KBytes = type { i64, ptr }

declare %KValue @k_int(i64)
declare %KValue @k_float(double)
declare %KValue @k_bool(i64)
declare %KValue @k_none()
declare %KValue @k_str_n(ptr, i64)
declare i64 @k_not_failure(%KValue)
declare %KValue @k_err(%KValue)
declare %KValue @k_rec(i64, i64, ptr)
declare %KValue @k_field(%KValue, i64)
declare %KValue @k_err_inner(%KValue)
declare i64 @k_check_tag(%KValue, i64)
declare i64 @k_check_int(%KValue, i64)
declare i64 @k_check_rec(%KValue, i64, i64)
declare i64 @k_check_bool(%KValue)
declare i64 @k_check_str(%KValue, ptr, i64)
declare %KValue @k_concat(%KValue, %KValue)
declare %KValue @k_render(%KValue, i64)
declare %KValue @k_add(%KValue, %KValue)
declare %KValue @k_sub(%KValue, %KValue)
declare %KValue @k_mul(%KValue, %KValue)
declare %KValue @k_div(%KValue, %KValue)
declare %KValue @k_cmp(%KValue, %KValue, i64)
declare %KValue @k_desc_print(%KValue)
declare %KValue @k_seq(%KValue, %KValue)
declare i64 @k_truthy(%KValue)
declare void @k_die(ptr) noreturn
declare { i64, i1 } @llvm.sadd.with.overflow.i64(i64, i64)
declare { i64, i1 } @llvm.ssub.with.overflow.i64(i64, i64)
declare { i64, i1 } @llvm.smul.with.overflow.i64(i64, i64)
declare %KValue @k_list_lit(i64, ptr)
declare %KValue @k_map_lit(i64, ptr)
declare %KValue @k_closure(ptr, i64, ptr)
declare %KValue @k_fnref(ptr)
declare %KValue @k_env_get(ptr, i64)
declare %KValue @k_b_at(%KValue, %KValue)
declare %KValue @k_index(%KValue, %KValue)
declare %KValue @k_b_bytes(%KValue)
declare %KValue @k_b_chars(%KValue)
declare %KValue @k_b_concat(%KValue, %KValue)
declare %KValue @k_b_utf8(%KValue)
declare %KValue @k_desc_args()
declare %KValue @k_desc_stdin()
declare %KValue @k_b_read_file(%KValue)
declare %KValue @k_b_write_file(%KValue, %KValue)
declare %KValue @k_maybe_bind(%KValue, %KValue)
declare %KValue @k_b_char_code(%KValue)
declare %KValue @k_b_entries(%KValue)
declare %KValue @k_b_filter(%KValue, %KValue)
declare %KValue @k_b_from_code(%KValue)
declare %KValue @k_b_join(%KValue, %KValue)
declare %KValue @k_b_length(%KValue)
declare %KValue @k_b_map(%KValue, %KValue)
declare %KValue @k_b_push(%KValue, %KValue)
declare %KValue @k_b_put(%KValue, %KValue, %KValue)
declare %KValue @k_b_slice(%KValue, %KValue, %KValue)
declare %KValue @k_b_sort(%KValue)
declare %KValue @k_b_sum(%KValue)
declare %KValue @k_b_to_float(%KValue)
declare %KValue @k_b_to_int(%KValue)

@s0 = private unnamed_addr constant [5 x i8] c"entry"
@s1 = private unnamed_addr constant [6 x i8] c"record"
@s2 = private unnamed_addr constant [7 x i8] c"ninth: "
@s3 = private unnamed_addr constant [46 x i8] c"no overload of `main` matches these arguments\00"

define ptr @k_type_name(i64 %id) {
entry:
  switch i64 %id, label %TD [
    i64 0, label %T0
  ]
T0:
  ret ptr @s0
TD:
  ret ptr @s1
}

define tailcc %KValue @d_main_0() {
entry:
  %t1 = alloca [3 x %KValue]
  %t2 = getelementptr [3 x %KValue], ptr %t1, i64 0, i64 0
  store %KValue { i64 0, i64 30 }, ptr %t2
  %t3 = getelementptr [3 x %KValue], ptr %t1, i64 0, i64 1
  store %KValue { i64 0, i64 10 }, ptr %t3
  %t4 = getelementptr [3 x %KValue], ptr %t1, i64 0, i64 2
  store %KValue { i64 0, i64 20 }, ptr %t4
  %t5 = call %KValue @k_list_lit(i64 3, ptr %t1)
  %t6 = call %KValue @k_str_n(ptr @s2, i64 7)
  %t7 = extractvalue %KValue %t5, 0
  %t8 = icmp eq i64 %t7, 13
  %t9 = extractvalue %KValue { i64 0, i64 9 }, 0
  %t10 = icmp eq i64 %t9, 0
  %t11 = and i1 %t8, %t10
  br i1 %t11, label %L1, label %L2
L1:
  %t12 = extractvalue %KValue %t5, 1
  %t13 = inttoptr i64 %t12 to ptr
  %t14 = getelementptr %KBytes, ptr %t13, i64 0, i32 0
  %t15 = load i64, ptr %t14
  %t16 = extractvalue %KValue { i64 0, i64 9 }, 1
  %t17 = icmp sge i64 %t16, 1
  %t18 = icmp sle i64 %t16, %t15
  %t19 = and i1 %t17, %t18
  br i1 %t19, label %L4, label %L2
L4:
  %t20 = getelementptr %KBytes, ptr %t13, i64 0, i32 1
  %t21 = load ptr, ptr %t20
  %t22 = add i64 %t16, -1
  %t23 = getelementptr i8, ptr %t21, i64 %t22
  %t24 = load i8, ptr %t23
  %t25 = zext i8 %t24 to i64
  %t26 = insertvalue %KValue { i64 0, i64 undef }, i64 %t25, 1
  br label %L3
L2:
  %t27 = call %KValue @k_index(%KValue %t5, %KValue { i64 0, i64 9 })
  br label %L3
L3:
  %t28 = phi %KValue [ %t26, %L4 ], [ %t27, %L2 ]
  %t29 = call %KValue @k_render(%KValue %t28, i64 0)
  %t30 = call %KValue @k_concat(%KValue %t6, %KValue %t29)
  %t31 = call %KValue @k_desc_print(%KValue %t30)
  ret %KValue %t31
fail0:
  call void @k_die(ptr @s3)
  unreachable
}

define %KValue @k_user_main() {
entry:
  %r = call tailcc %KValue @d_main_0()
  ret %KValue %r
}
