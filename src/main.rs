use std::ffi::c_void;
use std::rc::Rc;
use v8;

#[derive(Default)]
pub struct RustThing {
    pub val: i32,
    big: i128,
}

fn obj_constructor(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let this = args.this();

    let i = args.get(0);
    assert!(i.is_int32(), "expecting integer value");
    let ir = i.to_integer(scope).unwrap();
    let ir = ir.int32_value(scope).expect("couldn't get rust i32");

    let rust_obj = RustThing { val: -ir, big: 1 };
    let b = Box::new(rust_obj);

    // HELP: this is the source of the leak, but how to get v8.rs to clean up below?
    let wrapped_external = v8::External::new(scope, Box::into_raw(b) as *mut c_void);
    this.set_internal_field(0, wrapped_external.into());

    // HELP: This is never run. And what to put in there?
    // related to this: https://github.com/denoland/rusty_v8/blob/dd5fa705d430531ce4dd77605cec4adf2ed5ce80/tests/test_api.rs#L7399 ?
    // Must this be kept or sent out on a channel?
    Rc::new(v8::Weak::with_guaranteed_finalizer(
        scope,
        &this,
        Box::new(move || panic!("HELP: how to make it so this called?")),
    ));
    rv.set(this.into())
}

fn thing_val(
    scope: &mut v8::HandleScope,
    key: v8::Local<v8::Name>,
    args: v8::PropertyCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let s = key.to_rust_string_lossy(scope);
    assert!(s == "val");
    let this = args.this();

    let fld = this.get_internal_field(scope, 0).unwrap();
    let ext = unsafe { v8::Local::<v8::External>::cast(fld) };
    let rust_thing: &RustThing = unsafe { &*(ext.value() as *mut RustThing) };
    assert!(rust_thing.big == 1);

    rv.set_int32(rust_thing.val);
}

fn main() {
    let platform = v8::new_default_platform(2, false).make_shared();
    v8::V8::set_flags_from_string("--expose_gc");
    v8::V8::initialize_platform(platform);
    v8::V8::initialize();

    let mut isolate =
        v8::Isolate::new(v8::Isolate::create_params().heap_limits(2usize.pow(20), 2usize.pow(26)));

    let scope = &mut v8::HandleScope::new(&mut isolate);
    let context = v8::Context::new(scope);

    let scope = &mut v8::ContextScope::new(scope, context);

    let obj_class = v8::FunctionTemplate::new(scope, obj_constructor);
    obj_class
        .instance_template(scope)
        .set_internal_field_count(1);

    let name = v8::String::new(scope, "Thing").unwrap();
    obj_class.set_class_name(name);
    let val_string = v8::String::new(scope, "val").unwrap();
    obj_class
        .instance_template(scope)
        .set_accessor(val_string.into(), thing_val);

    let obj = obj_class.get_function(scope).unwrap();
    context.global(scope).set(scope, name.into(), obj.into());

    let js = r"
var j
for (i = 0; i < 4000; i++){
    for (k = 0; k < 1000; k++) {
        j = new Thing(i);
    }
    gc();
}
j.val
";
    let js = v8::String::new(scope, js).unwrap();
    let script = v8::Script::compile(scope, js, None).unwrap();
    let result = script.run(scope).unwrap();
    let result = result.to_rust_string_lossy(scope);
    println!("result: {:?}", result);
}
