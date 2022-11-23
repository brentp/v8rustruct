
This creates an example using v8.rs where a JS object (`Thing`) attaches a rust struct (`RustThing`) inside
of `obj_constructor`. It then accesses that rust struct in the accessor `thing_val`. 

This setup results in a memory leak. I understand why, but not how to fix.

There are several instances of "HELP" in `src/main.rs` where I don't understand how to resolve the leak. 

