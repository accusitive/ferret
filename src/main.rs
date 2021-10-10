#![feature(iter_advance_by)]
use std::{path::Path, rc::Rc};

use cafebabe::constant_pool::{ConstantPoolEntry, ConstantPoolItem};
use inkwell::{
    context::Context,
    module::Linkage,
    targets::{CodeModel, InitializationConfig, RelocMode, Target, TargetMachine},
    types::{AnyType, AnyTypeEnum, BasicType, BasicTypeEnum, FunctionType},
    values::{AnyValueEnum, ArrayValue, BasicValueEnum},
    OptimizationLevel,
};
use jni::signature::JavaType;

// use std::fs::File;

// use jvm_assembler::*;
// fn main() {
//     let f = File::open("Hello.class").unwrap();
//     let class = jvm_assembler::Classfile::deserialize(Box::new(f));

//     for method in &class.methods {
//         println!(
//             "Method index {}\nMethod name {:?}",
//             method.name_index,
//             class
//                 .constant_pool
//                 .get((method.name_index - 1) as usize)
//                 .unwrap()
//         );
//         println!("Class {}", class);

//         for attrib in method.attributes.get(0) {
//             match attrib {
//                 Attribute::Code(
//                     _,
//                     max_stack,
//                     max_locals,
//                     instructions,
//                     exception_table_entry,
//                     attributes_,
//                 ) => {
//                     for instruction in instructions {
//                         println!("ins {:?}", instruction);
//                     }
//                 }
//                 Attribute::LineNumberTable(_, _) => todo!(),
//                 Attribute::SourceFile(_, _) => todo!(),
//                 Attribute::StackMapTable(_, _) => todo!(),
//             }
//         }
//     }
// }
// /*
//     Get static System.out.
//     Load constant Jwalus
//     Invoke PrinStream println

// */
#[no_mangle]
fn xnew() -> Stack {
    println!("test");
    Stack {
        data: [0; 1024],
        top: 0,
    }
}

fn main() {
    let class = cafebabe::parse_class(include_bytes!("../Hello.class")).unwrap();
    println!("Class {:#x?}", class);
    // class.attributes.push(AttributeInfo {
    //     name: std::borrow::Cow::Borrowed("SourceFile"),
    //     data: AttributeData::SourceFile(std::borrow::Cow::Borrowed(include_str!(
    //         "../HelloWorld.java"
    //     ))),
    // });
    Target::initialize_all(&InitializationConfig::default());
    Target::initialize_native(&InitializationConfig::default()).unwrap();

    let ctx = Context::create();
    let module = ctx.create_module("module");
    let builder = ctx.create_builder();

    // let execution_engine = module
    //     .create_jit_execution_engine(OptimizationLevel::None)
    //     .unwrap();

    let int32 = ctx.i32_type();
    let void = ctx.void_type();

    let stack_ty = {
        let arr = int32.array_type(1024);
        ctx.struct_type(&[arr.into(), int32.into()], false)
    };

    let varstore_ty = {
        let arr = int32.array_type(1024);
        ctx.struct_type(&[arr.into()], false)
    };
    let std_stack_new = module.add_function(
        "stack_new",
        stack_ty
            .ptr_type(inkwell::AddressSpace::Global)
            .fn_type(&[], false),
        Some(Linkage::External),
    );

    let std_stack_push = module.add_function(
        "stack_push",
        void.fn_type(&[stack_ty.into()], false),
        Some(Linkage::External),
    );
    let std_stack_pop = module.add_function(
        "stack_pop",
        int32.fn_type(&[stack_ty.into()], false),
        Some(Linkage::External),
    );

    let varstore_new = module.add_function(
        "varstore_new",
        varstore_ty
            .ptr_type(inkwell::AddressSpace::Global)
            .fn_type(&[], false),
        Some(Linkage::External),
    );

    let varstore_set = module.add_function(
        "varstore_set",
        void.fn_type(
            &[
                inkwell::types::BasicTypeEnum::StructType(varstore_ty),
                int32.into(),
                int32.into(),
            ],
            false,
        ),
        Some(Linkage::External),
    );
    let varstore_get = module.add_function(
        "varstore_get",
        int32.fn_type(
            &[
                inkwell::types::BasicTypeEnum::StructType(varstore_ty),
                int32.into(),
            ],
            false,
        ),
        Some(Linkage::External),
    );
    // let varstore_set = module.add_function(
    //     "varstore_set",
    //     void.fn_type(&[int32().into(), in32().into()], false),
    //     Some(Linkage::External),
    // );
    fn jnity_to_llvm_ty<'a>(ctx: &'a Context, j: &JavaType) -> AnyTypeEnum<'a> {
        match j {
            jni::signature::JavaType::Primitive(p) => match p {
                jni::signature::Primitive::Boolean => ctx.bool_type().as_any_type_enum(),
                jni::signature::Primitive::Byte | jni::signature::Primitive::Char => {
                    ctx.i8_type().as_any_type_enum()
                }
                jni::signature::Primitive::Double => ctx.f64_type().as_any_type_enum(),
                jni::signature::Primitive::Float => ctx.f32_type().as_any_type_enum(),
                jni::signature::Primitive::Int => ctx.i32_type().as_any_type_enum(),
                jni::signature::Primitive::Long => ctx.i64_type().as_any_type_enum(),
                jni::signature::Primitive::Short => ctx.i16_type().as_any_type_enum(),
                jni::signature::Primitive::Void => ctx.void_type().as_any_type_enum(),
            },
            jni::signature::JavaType::Object(o) => ctx.i32_type().as_any_type_enum(),
            jni::signature::JavaType::Array(_) => todo!(),
            jni::signature::JavaType::Method(_) => todo!(),
        }
    }
    fn parse_method_type(ctx: &Context, d: String) -> (usize, FunctionType) {
        let s = jni::signature::TypeSignature::from_str(d).unwrap();
        // println!("SIG {:?}", s);
        // BasicTypeEnum::
        // ret.fn_type(param_types, is_var_args)
        let params = &s
            .args
            .iter()
            .map(|a| jnity_to_llvm_ty(&ctx, a).try_into().unwrap())
            .collect::<Vec<_>>();

        let ret = jnity_to_llvm_ty(&ctx, &s.ret);
        if let AnyTypeEnum::VoidType(v) = ret {
            // let k: BasicTypeEnum = v;

            let t = v.fn_type(&params, false);
            (0, t)
        } else {
            let t: BasicTypeEnum = ret.try_into().unwrap();
            let f = t.fn_type(&params, false);

            (0, f)
        }
    }
    for method in &class.methods {
        if method.name == "<init>" {
            continue;
        }
        // let add_ty = int32.fn_type(&[int32.into(), int32.into(), int32.into()], false);
        // let times_two_ty = int32.fn_type(&[int32.into()], false);
        // let fty = match method.name.to_string().as_str() {
        //     "add" => add_ty,
        //     "timesTwo" => times_two_ty,
        //     _ => panic!(),
        // };
        let linkage = Linkage::WeakAny;
        let method_sig = parse_method_type(&ctx, method.descriptor.to_string());

        let function = match module.get_function(&method.name) {
            Some(f) => f,
            None => module.add_function(&method.name, method_sig.1, Some(linkage)),
        };
        // let function = ;
        println!("Compiling function {}", &method.name);

        // let std_stack_push = module.get_function("stack_push").unwrap();

        for attrib in &method.attributes {
            // if attrib.name == "Code" {
            match &attrib.data {
                cafebabe::attributes::AttributeData::ConstantValue(_) => todo!(),
                cafebabe::attributes::AttributeData::Code(code) => {
                    println!("Code {:#x?} descrip {}", code, method.descriptor);
                    let entry = ctx.append_basic_block(function, "entry");
                    builder.position_at_end(entry);

                    let _local0 = builder.build_alloca(int32, "local0"); // this
                    let local1 = builder.build_alloca(int32, "local1");
                    let local2 = builder.build_alloca(int32, "local2");
                    let local3 = builder.build_alloca(int32, "local3");

                    // Copy params over. 1 <- 0 because local0 is `this`.
                    builder.build_store(local1, function.get_nth_param(0).unwrap());
                    if method_sig.0 > 1 {
                        builder.build_store(local2, function.get_nth_param(1).unwrap());
                    }
                    if method_sig.0 > 2 {
                        builder.build_store(local3, function.get_nth_param(2).unwrap());
                    }

                    // builder.build_store(local3, function.get_nth_param(3).unwrap());

                    let stack = builder.build_call(std_stack_new, &[], "stack");
                    let varstore = builder.build_call(varstore_new, &[], "varstore");

                    function.set_call_conventions(0); // c
                    let mut peekable_code = code.code.iter().peekable();
                    // while let Some(by) = peekable_code.next() {
                    //     let next = peekable_code.peek();
                    // }
                    // for by in code.code {
                    while let Some(by) = peekable_code.next() {
                        match by {
                            0x3..=0x08 => {
                                let one = int32.const_int((by - 3).into(), false);
                                builder.build_call(
                                    std_stack_push,
                                    &[stack.try_as_basic_value().left().unwrap(), one.into()],
                                    "push_il1",
                                );
                            }
                            0x1b => {
                                let l1 = builder.build_load(local1, "iload_1");
                                builder.build_call(
                                    std_stack_push,
                                    &[stack.try_as_basic_value().left().unwrap(), l1],
                                    "push_il1",
                                );
                            }
                            0x1c => {
                                let l2 = builder.build_load(local2, "iload_2");
                                builder.build_call(
                                    std_stack_push,
                                    &[stack.try_as_basic_value().left().unwrap(), l2],
                                    "push_il2",
                                );
                            }
                            0x1d => {
                                let l3 = builder.build_load(local3, "iload_3");
                                builder.build_call(
                                    std_stack_push,
                                    &[stack.try_as_basic_value().left().unwrap(), l3],
                                    "push_il3",
                                );
                            }
                            0x36 => {
                                let index = peekable_code.next().unwrap();
                                let val = builder.build_call(
                                    std_stack_pop,
                                    &[stack.try_as_basic_value().left().unwrap()],
                                    "val",
                                );
                                builder.build_call(
                                    varstore_set,
                                    &[
                                        varstore.try_as_basic_value().left().unwrap(),
                                        inkwell::values::BasicValueEnum::IntValue(
                                            int32.const_int(*index as u64, false),
                                        ),
                                        val.try_as_basic_value().left().unwrap(),
                                    ],
                                    "value",
                                );
                                // builder.build_call(varstore_set, &[index,], name)
                                // variables[]
                            }
                            0x15 => {
                                let index = peekable_code.next().unwrap();
                                let value = builder.build_call(
                                    varstore_get,
                                    &[
                                        varstore.try_as_basic_value().left().unwrap(),
                                        inkwell::values::BasicValueEnum::IntValue(
                                            int32.const_int(*index as u64, false),
                                        ),
                                    ],
                                    "value",
                                );
                                builder.build_call(
                                    std_stack_push,
                                    &[
                                        stack.try_as_basic_value().left().unwrap(),
                                        value.try_as_basic_value().unwrap_left(),
                                    ],
                                    "iload",
                                );
                            }
                            0x60 => {
                                let lhs = builder.build_call(
                                    std_stack_pop,
                                    &[stack.try_as_basic_value().left().unwrap()],
                                    "lhs",
                                );
                                let rhs = builder.build_call(
                                    std_stack_pop,
                                    &[stack.try_as_basic_value().left().unwrap()],
                                    "rhs",
                                );
                                let sum = builder.build_int_add(
                                    lhs.try_as_basic_value().left().unwrap().into_int_value(),
                                    rhs.try_as_basic_value().left().unwrap().into_int_value(),
                                    "sum",
                                );
                                builder.build_call(
                                    std_stack_push,
                                    &[
                                        stack.try_as_basic_value().left().unwrap(),
                                        inkwell::values::BasicValueEnum::IntValue(sum),
                                    ],
                                    "pushed_iadd",
                                );
                            }
                            0x68 => {
                                let lhs = builder.build_call(
                                    std_stack_pop,
                                    &[stack.try_as_basic_value().left().unwrap()],
                                    "lhs",
                                );
                                let rhs = builder.build_call(
                                    std_stack_pop,
                                    &[stack.try_as_basic_value().left().unwrap()],
                                    "rhs",
                                );
                                let sum = builder.build_int_mul(
                                    lhs.try_as_basic_value().left().unwrap().into_int_value(),
                                    rhs.try_as_basic_value().left().unwrap().into_int_value(),
                                    "product",
                                );
                                builder.build_call(
                                    std_stack_push,
                                    &[
                                        stack.try_as_basic_value().left().unwrap(),
                                        inkwell::values::BasicValueEnum::IntValue(sum),
                                    ],
                                    "pushed_imul",
                                );
                            }
                            0xac => {
                                let popped = builder.build_call(
                                    std_stack_pop,
                                    &[stack.try_as_basic_value().left().unwrap()],
                                    "popped",
                                );

                                builder.build_return(Some(
                                    &popped.try_as_basic_value().left().unwrap(),
                                ));
                            }
                            0x2a => {
                                let l1 = builder.build_load(local1, "aload_1");
                                builder.build_call(
                                    std_stack_push,
                                    &[stack.try_as_basic_value().left().unwrap(), l1],
                                    "push_il1",
                                );
                            }
                            // 0x2a => {
                            //     //TODO: I dont think this is correct
                            //     let pv = local0;
                            //     let loaded = builder.build_load(pv, "loading");
                            //     builder.build_call(
                            //         std_stack_push,
                            //         &[
                            //             stack.try_as_basic_value().left().unwrap(),
                            //             loaded,
                            //         ],
                            //         "aload_0",
                            //     );
                            // }
                            0xb6 => {
                                let index_byte_1 = peekable_code.next().unwrap();
                                let index_byte_2 = peekable_code.next().unwrap();
                                println!("Index 1 {} index 2 {}", index_byte_1, index_byte_2);
                                let obj = builder.build_call(
                                    std_stack_pop,
                                    &[stack.try_as_basic_value().left().unwrap()],
                                    "objpop",
                                );
                                let mut i = class.constantpool_iter();
                                i.advance_by((*(index_byte_2) - 1) as usize).unwrap();
                                let method_ref = i.next().unwrap();
                                match method_ref {
                                    ConstantPoolItem::MethodRef {
                                        class_name: _,
                                        name_and_type,
                                    } => {
                                        let ty = parse_method_type(
                                            &ctx,
                                            name_and_type.descriptor.to_string(),
                                        )
                                        .1;
                                        println!("Adding function named {}", name_and_type.name);
                                        let llvm_func = module.add_function(
                                            &name_and_type.name,
                                            ty,
                                            Some(linkage),
                                        );

                                        // llvm_func.get
                                        // builder.build_call(
                                        //     llvm_func,
                                        //     &[obj.try_as_basic_value().unwrap_left()],
                                        //     "call_for_invoke_virtual",
                                        // );
                                        let loaded = builder.build_load(local1, "load1");
                                        let built = builder.build_call(
                                            llvm_func,
                                            &[obj.try_as_basic_value().unwrap_left(), loaded],
                                            "call_for_invoke_virtual",
                                        );

                                        builder.build_call(
                                            std_stack_push,
                                            &[
                                                stack.try_as_basic_value().left().unwrap(),
                                                // built.try_as_basic_value().unwrap_right(),
                                            ],
                                            "push_il1",
                                        );
                                    }

                                    _ => panic!("Method ref is not method ref???"),
                                }
                                // println!("{:#?}", method_ref);
                            }
                            0x10 => {
                                let bi = peekable_code.next().unwrap();
                                builder.build_call(
                                    std_stack_push,
                                    &[
                                        stack.try_as_basic_value().left().unwrap(),
                                        inkwell::values::BasicValueEnum::IntValue(
                                            int32.const_int(*bi as u64, false),
                                        ),
                                    ],
                                    "push_il1",
                                );
                            }
                            0x3d => {
                                let popped = builder.build_call(
                                    std_stack_pop,
                                    &[stack.try_as_basic_value().left().unwrap()],
                                    "popped",
                                );
                                builder
                                    .build_store(local2, popped.try_as_basic_value().unwrap_left());
                            }
                            0xb2 => {
                                let index_byte_1 = peekable_code.next().unwrap();
                                let index_byte_2 = peekable_code.next().unwrap();
                                println!("Index 1 {} index 2 {}", index_byte_1, index_byte_2);
                                // let obj = builder.build_call(
                                //     std_stack_pop,
                                //     &[stack.try_as_basic_value().left().unwrap()],
                                //     "objpop",
                                // );
                                let mut i = class.constantpool_iter();
                                i.advance_by((*(index_byte_2) - 1) as usize).unwrap();
                                let statik = i.next().unwrap();
                                if let ConstantPoolItem::FieldRef {
                                    class_name,
                                    name_and_type,
                                } = statik
                                {
                                    
                                    let cnst = class.constant_pool.get(*index_byte_2 as usize).unwrap();
                                    // cnst.
                                    todo!()
                                    // cnst.
                                    // class.attributes.get()
                                    // constant
                                }
                            }
                            0x12 => {
                                let index = peekable_code.next().unwrap();
                                let mut i = class.constantpool_iter();
                                let mut statik = i.next().unwrap();
                                let constant = class.constant_pool.get(*index as usize).unwrap();
                                // println!("CONSTANT {:#?}", constant);
                                // if let Rc::new(ConstantPoolEntry::String(cpr)) = constant{
                                //     builder.build_call(
                                //         std_stack_push,
                                //         &[
                                //             stack.try_as_basic_value().left().unwrap(),
                                //             inkwell::values::BasicValueEnum::VectorValue(
                                //                 ctx.const_string(&constant.name_and_type().name.as_bytes(), false),
                                //             ),
                                //         ],
                                //         "push_il1",
                                //     );
                                // }
                                match constant.as_ref() {
                                    cafebabe::constant_pool::ConstantPoolEntry::String(rcthing) => {
                                      match &*rcthing.borrow() {
                                        cafebabe::constant_pool::ConstantPoolRef::Unresolved(_) => todo!(),
                                        cafebabe::constant_pool::ConstantPoolRef::Resolved(r) => {
                                                builder.build_call(
                                        std_stack_push,
                                        &[
                                            stack.try_as_basic_value().left().unwrap(),
                                            inkwell::values::BasicValueEnum::VectorValue(
                                                ctx.const_string(&r.utf8().to_string().as_bytes(), false),
                                            ),
                                        ],
                                        "push_il1",
                                            );
                                            },
                                         }
                                    }
                                    _ => {}
                                }
                            }
                            _ => panic!("Byte {:x} not implemented!", by),
                        }
                    }
                    if method.name == "<init>" {
                        let x = function.get_nth_param(0).unwrap().into_int_value();
                        let y = function.get_nth_param(1).unwrap().into_int_value();
                        let sum = builder.build_int_add(x, y, "sum");
                        builder.build_return(Some(&sum));
                        // }
                    }
                }
                _ => todo!(),
            }
            // }`
        }
    }
    module.print_to_stderr();
    // module.li
    // execution_engine.add_global_mapping(value, addr)
    // module.ini
    // let adder = unsafe { execution_engine.get_function::<unsafe extern fn(i32,i32,i32) -> i32>("add") }.unwrap();
    // let called = unsafe { adder.call(1, 2, 5) };
    // println!("Called {}", called);
    // println!("hmmmm");
    // module.
    let target_triple = TargetMachine::get_default_triple();
    let target = Target::from_triple(&target_triple).unwrap();
    let t = target
        .create_target_machine(
            &target_triple,
            &get_host_cpu_name(),
            &get_host_cpu_features(),
            OptimizationLevel::Aggressive,
            RelocMode::Default,
            CodeModel::Default,
        )
        .unwrap();
    t.write_to_file(
        &module,
        inkwell::targets::FileType::Object,
        Path::new("./out.o"),
    )
    .unwrap();
    t.write_to_file(
        &module,
        inkwell::targets::FileType::Assembly,
        Path::new("./out.s"),
    )
    .unwrap();
}
fn get_host_cpu_name() -> String {
    TargetMachine::get_host_cpu_name().to_string()
}
fn get_host_cpu_features() -> String {
    TargetMachine::get_host_cpu_features().to_string()
}
// #![allow(non_upper_case_globals)]
// #![allow(non_camel_case_types)]
// #![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
