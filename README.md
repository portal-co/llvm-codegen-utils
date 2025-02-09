# LLVM Codegen Utils

A new library to facilitate targetting LLVM, with a mostly safe interface. (Handles currently do not accurately represent their ownership.)

## Comparison with `inkwell`

`inkwell` exposes, safely, the actual, non-modified, LLVM API. LLVM Codegen Utils exposes and/or wraps the API to present a safe interface, at the cost of using a slightly higher-level and/or cleaned interface.

`inkwell` only supports LLVM 18 and lower, while LLVM Codegen Utils supports LLVM 18 and higher.