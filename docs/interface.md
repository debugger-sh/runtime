# Interface

## Desiderata

The interface we should provide should:

* Accept code from one of several langauges (e.g. `Python`, `C`. `C++`, `Rust`, etc.), which can grow over time as the library evolves, all of which share a common runtime interface.
* Transparently communicate execution errors to the client (compiler, linker, user errors, etc).
* Indicate the phase of execution of the user code (preparing, compiling, linking, running, done).
* Provide methods for interfacing with standard in, out, and error.
* Allow configuring the initial state of the virtual filesystem.
* Provide a debugging interface which allows:
  - Reading/modifying which breakpoint locations have been set
  - Stepping into, out of, or over the current stop point
  - On break, reading the stack trace with the state of locals for each frame