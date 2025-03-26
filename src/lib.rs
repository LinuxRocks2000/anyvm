// simple, embeddable VM and associated compiler system.
// rather than an LLVM-like intermediate representation, AnyVM directly turns an AST in memory into bytecode and also executes bytecode.
// anyvm machine images (which contain a a static section and code) can be dumped to files and loaded from files
// anyvm is designed around the needs of PCP.
// anyvm machines are always 64-bit big-endian.

/* struct Image
    machine image. contains a symbol lookup table, a static section, and a text section
    to execute an image, you need to mount it to a Machine and look up functions to call on it (common is `main`)
*/

// note that the lookup tables do NOT need to contain every symbol, and can in fact be empty! they're only for *public* symbols.

// in vm memory, the static section always starts at the front. all memory including static memory is mutable - this allows the program to edit itself!
// (note that languages using anyvm SHOULD implement rust-like mutability controls. this is not because of any property of the vm [although some
// optimizations may become simpler], but because rust's mutability controls are Good™ and C-style mutability is Bad™)

// all pointers are i64s. positive addresses are absolute indexes into vm memory; negative addresses are relative indexes from the top of the stack.
// this makes many things simpler.

// "rabbit addresses" are memory addresses beyond the bounds of the VM. they are used by the hypervisor to implement special behavior.
// they're called that because, like the magician's rabbit, at first sight they appear strange and mysterious, but are in fact quite mundane and simple.
// operations that produce rabbit addresses should be treated as untrustworthy black boxes. calling them many times is a bad idea. their behavior produces
// *no* guarantees beyond what is explicitly stated - they may take arbitrarily long, the rabbit addresses may be anything (as long as they're not inside VM memory),
// the rabbit allocator may decide to reuse rabbit addresses, etc.

/*
    opcodes:
    // note that many of these are generic over architecture-supported sizes: l = 64 bit, i = 32 bit, s = 16 bit, b = 8 bit.
    formatting: (a, b, c) optional a, b, OR c
                [a, b, c] required a, b, OR c

    // basic memory operations (pushing, popping, swapping, moving)
    0 -> 7. push(v)[l, i, s, b]: (v): push a value to the stack, OR push data from somewhere else in memory to stack
    8 -> 11. swap[l, i, s, b]: swap two values in memory
    12 -> 19. cpy(v)[l, i, s, b]: (v): set a value in memory, OR copy a value from somewhere else in memory.
    20 -> 23. pop[l, i, s, b]: pop a value from stack and discard it
    24 -> 27. popm[l, i, s, b]: pop a value from stack to a point in memory

    // int arithmetic
    28 -> 31. add[l, i, s b]: add two points in memory (the result will overwrite the first point)
    32 -> 35. sub[l, i, s, b]: subtract the second point from the first point (same overwrite semantics as add)
    36 -> 39. mul[l, i, s, b]: multiply. same semantics as add,sub
    40 -> 43. div[l, i, s, b]: divide. same semantics as above

    // note: I haven't yet checked if bitwise division actually does work the same for signed and unsigned values. oops.
    // may need to expand this to (i)div(v)[l, i, s, b]

    // logical bitwise operations
    44 -> 51. cmp(v)[l, i, s, b]: compare values in memory. Get the second value either (v) from the instruction or from elsewhere in memory.
        Push the 1-byte result to the stack:
        if they're equal, 0
        if one is greater than two, 1
        if two is greater than one, 2
    52: bnot: flip all the bits of a byte in memory.
    53: not: replace a byte in memory with 1 if it is 0, or 0 otherwise
    54: bor: take a bitwise OR of two bytes in memory and replace the first byte with the result.
    55: vor: compare a byte in memory with a specified value. same semantics as bor.
    56: band: take a bitwise AND of two byte in memory and replace the first byte with the result
    57: vand: compare a byte in memory with a specified value. same semantics as band.
    58 -> 61: shift[l, i, s, b]: bitshift a value in memory by some specified amount (the amount should be a signed 8-bit int)
    62: bnorm: if a byte in memory is not 0, set it to 1, otherwise set it to 0.

    // flow control
    63. jmp: Increment or decrement the execution pointer by the signed 64-bit int argument.
    64. branch: if the value in a specified register is 0, branch to a specified location. else, continue with the next operation.
        location is an absolute op location
    65. call: call a function: absolute version of jmp, but it pushes a return address to stack first.
        you have to push the arguments to stack *before* `call`ing, and the function must still handle stack allocating its own local variables.
        the first stack push a caller makes should be reserving space for the function's return value, if any.
    66. ret: return from a function. expects the top value on the stack to be the return address - that is, the callee function has to unwind the stack down to the return address
        before calling ret.
    67. invokevirtual: `call`, except it dereferences the argument to a 64-bit value somewhere in memory.
    68. invokeext: invoke an external function (loaded by way of a table)
        To avoid bad recursions, invokeext ALWAYS sets sbm to 0. Attempting to use invokeext
        without checkerr will lead to undefined behavior.
    69. setsbm: set a stack break marker. this will push the previous value of the sbm pointer to stack (0 if there is no current sbm)
        meant to be used in conjunction with checkerr.
        the sbm is actually two pointers: the execution pointer and the stack pointer. this means it takes up 16 bytes in memory.
        when setsbm executes, the stack pointer is stored in the sbm unaltered, and the execution pointer is stored with a 9-byte increment to skip
        over a call, invokevirtual, or invokeext. this means that any fallible functions should be called like
         * setsbm
         * call <function>
         * checkerr <handler_location>
        the default SBM is all 0s.
    70. throw: throw an error. accepts an 8-bit error reason. throw is mostly used by the ABI in situations where a proper error handler would not work.
        when an error is thrown, the stack and execution pointer are rewound to SBM, and the SBM is reset to the SBM pushed on the top of the stack.
        The sbm is not popped off the stack; it should be popped off with checkerr.
        If the SBM is all 0, this will fully abort the vm.
        error codes:
         0: nerr; no error occurred, why are you geterr'ing?
         1: out-of-bounds memory access.
         2: out-of-bounds function call.
         3: table lookup failure.
         4: table allocation failure.
        == Please for the love of all that is holy do not use throw in normal situations. It should only ever be used in cases where proper enumerated
        == error handling is utterly impossible, like if the user attempts to execute an invalid external function pointer.
        == Why does it even exist?
        == I put some thought into this. It would be very nice to live in a world without stack unwinding, *but* there's a hitch: external functions 
        == don't have a well-defined calling convention; that is, they can treat the stack however they want, and oftentimes this is *useful*
        == (for instance, for extending the abi with rust functions). But this means that it's impossible to guarantee any convention for external return
        == values, so wrapping with an enum is simply unfeasible. Essentially: after calling such a function, your program has unpredictable expectations
        == about the state of the stack. The SBM pattern forces your program to either be aware that an error can be thrown and handle it properly,
        == or abort.
        == This pattern also allows bytecode operations to fail gracefully; rather than aborting the VM upon an out-of-bounds memory access, for instance,
        == it can be properly handled by some user-defined routine, which (critically!) avoids stack corruption.

        The thrown error code will be saved until the next instruction. The only instruction that will not overwrite the error code is checkerr.
    71. checkerr: if an error was thrown (error code is nonzero), jump to the specified location. Otherwise, continue to the next instruction.
        checkerr pops the SBM off the stack.
    72. geterr: push the last thrown error code to stack.

    // vm commands
    73. exit: exit the VM
    74. startmmu: start the MMU. this will create a page table at the end of the memory block. it's possible to do allocations without mmu
        just by directly editing memory, but using the mmu is better.
        startmmu requires a page size in bytes. larger page sizes means more wasted memory from each alloc
        call, but also means a smaller page table and less likelihood of having to move memory on realloc; choose wisely.
    75. alloc: allocate some bytes in VM memory. pops the (64-bit) number of bytes from stack and pushes the pointer.
        alloc may use (much) more memory than requested based on the page size.
    76. dealloc: free some bytes. pops the address from stack. must be page-aligned. you do not have to pass the length.
    77. realloc: reallocate some bytes. pops the address from stack, copies those pages out of VM memory, deallocates them,
        allocates a new chunk, and copies the pages back into that new chunk. If you choose a smaller value than the original allocation, the reallocation
        will be truncated.
    78. maketbl: tables are magical data structures. They look roughly like hashmaps indexed by strings. Table data is runtime-typed along some simple types
        and is dynamic in size. Tables are stored entirely in VM memory, and are accessible from outside the VM; they provide a convenient and safe interface
        between the VM application and the ABI.

        Do not directly mess with table memory. The layout is not predictable.
    79. pushtbl: push some data to a table. The top 64 bits on stack must be a pointer to the name as a null-terminated string. The next 64 bits must
        be a pointer to the actual table. The next one byte must be the type of the data, and the rest of the stack must be prepared correctly based on
        the type. Pushtbl may reallocate and always pushes the most recent pointer to the table to the stack.
        types:
            0 -> 3: 64, 32, 16, or 8 bit int.
            4: string (with 64 bits of length preceding) (this will COPY the string - be careful about memory!)
            5: table
            6: VM function
            7: external function
    80. gettbl: read some data from a table. the top 8 bytes must be the string index, and the next 8 bytes the pointer to the table.
        the type and appropriate data will be pushed to stack, type *last*. The data will always be exactly 64 bits, and its format is defined
        by type:
            0 -> 3: raw number data
            4: pointer to the string
            5: pointer to the table
            6: pointer to the bytes in memory representing the function
            7: external function id
    81. deltbl: delete an item in a table. the top 8 bytes must be a pointer to the index string, and the next 8 bytes the pointer to the table.
        this may reallocate and will push the most recent table pointer to the stack.
        deltbl will always free the memory in the table. If the data is a function it will not attempt to free the function. If the data is a string,
        it will free the string. If the data is a table, it will call freetbl.
    82. freetbl: delete every item in a table and free the table itself.
    83. updstck: change the stack pointer by an amount.
        TODO: move this near push and pop

    As yet there is no "native" floating-point support in anyvm.

    There are no registers in anyvm. Why is this?
    Registers make sense in actual processors because they're *very, very* fast. RAM, even L1 cache, is *much* slower than processor registers.
    However, because emulated registers would be stored in RAM regardless, registers are entirely pointless for anyvm.
*/

use std::collections::HashMap;

mod numerical;
use numerical::*;


use std::fmt::Debug;
pub mod invoke;


pub mod error;
use error::*;


pub mod ir;
pub mod avc;


pub struct Image {
    function_table : HashMap<String, i64>, // contains offsets into the text section.
    static_table : HashMap<String, i64>, // contains offsets into the static section
    static_section : Vec<u8>,
    text_section : Vec<u8> // bytecode. contains a bunch of functions crammed together.
}


impl Image {
    pub fn lookup(&self, thing : String) -> i64 {
        self.static_section.len() as i64 + self.function_table.get(&thing).unwrap() // todo: throw an error, rather than panicking
    }
}


pub trait Table {
    fn lookup(data : &str) -> ExtData;
}


pub enum ExtData {
    Function(Box<dyn FnMut<(&mut Machine)>>),
    Table(Box<dyn Table>)
}


pub struct Machine {
    memory : Vec<u8>,
    text_start : i64,
    stack_start : i64,
    end : i64,
    ext_data : Vec<ExtData>,
    stack_pointer : i64,
    exec_pointer : i64,
    errcode : u8,
    sbm : (i64, i64) // (stack, exec): stack break marker
}


impl Machine {
    pub fn new(capacity : usize) -> Machine {
        Machine {
            memory : vec![0u8; capacity],
            end : capacity as i64 - 8, // 8 byte padding at the end. why? to save a tonne of cycles. more below.
            stack_start : 0,
            text_start : 0,
            ext_data : vec![],
            stack_pointer : 0,
            exec_pointer : 0,
            sbm : (0, 0),
            errcode : 0
        }
    }

    pub fn mount(&mut self, image : &Image) {
        let mut head = self.memory.iter_mut();
        let mut ss = image.static_section.iter();
        let mut ts = image.text_section.iter();
        while let Some(byte) = ss.next() {
            *head.next().unwrap() = *byte; // TODO: throw OOM rather than panicking
        }
        while let Some(byte) = ts.next() {
            *head.next().unwrap() = *byte;
        }
        self.text_start = image.static_section.len() as i64;
        self.stack_start = self.text_start + image.text_section.len() as i64;
    }

    unsafe fn memory_as_at<'t, T>(&'t mut self, pos : usize) -> MemResult<&'t mut [T]> {
        if pos < self.memory.len() {
            Ok(std::mem::transmute::<&mut [u8], &mut [T]>(&mut self.memory[pos..]))
        }
        else {
            Err(MemoryErr::SegmentationFault)
        }
    }

    fn next_rabbit(&mut self) -> i64 {
        self.rabbit_top += 1;
        self.rabbit_top
    }

    fn stackaddr(&self, mut addr : i64) -> MemResult<usize> { // note how this doesn't actually check typed alignment,
        // meaning it's possible to dereference capacity - 1 as a u64, and peek into the 7 bytes *afterwards*
        // (which would cause a panic). This is avoided by simply adding 8 bytes of padding at the end of the memory block.
        // exhaustive checking is *possible*, but ultimately expensive and bug-prone; this system maximizes the speed of accesses
        // without compromising the hypervisor: a hacker *can* read past the end of memory, but won't see anything useful and won't panic the hypervisor.
        if addr < 0 {
            addr += self.stack_pointer
        }
        if addr < 0 || addr >= self.end {
            Err(MemoryErr::SegmentationFault)
        }
        else {
            Ok(addr as usize)
        }
    }

    fn get_at_as<T : Numerical>(&mut self, pos : i64) -> MemResult<T> {
        let pos = self.stackaddr(pos)?;
        Ok(unsafe {
            self.memory_as_at::<T>(pos)?[0].from_be()
        })
    }

    fn setmem<T : Numerical>(&mut self, pos : i64, val : T) -> MemResult<T> {
        let pos = self.stackaddr(pos)?;
        unsafe {
            self.memory_as_at::<T>(pos)?[0] = val.to_be();
        }
        Ok(val)
    }

    fn pop_arg<T : Numerical>(&mut self) -> MemResult<T> { // pop an arg
        let ret = self.get_at_as(self.stack_pointer);
        self.exec_pointer += T::BYTE_COUNT as i64;
        Ok(ret.from_be())
    }

    fn pop_arg_addr(&mut self) -> MemResult<usize> { // pop an argument and convert it to a stackaddr
        let arg = self.pop_arg::<i64>()?;
        self.stackaddr(arg)
    }

    fn pop_as<T : Numerical>(&mut self) -> MemResult<T> { // pop a thing off stack
        let r = self.get_at_as::<T>(-(T::BYTE_COUNT as i64));
        self.stack_pointer -= T::BYTE_COUNT as i64;
        r
    }

    fn push<T : Numerical>(&mut self, thing : T) -> MemResult<()> { // push a thing to stack
        self.setmem(0, thing);
        self.stack_pointer += T::BYTE_COUNT as i64;
        Ok(())
    }

    fn pop_addr(&mut self) -> MemResult<usize> { // pop an address off stack and run it through stackaddr()
        let vm_addr = self.pop_as::<i64>()?;
        self.stackaddr(vm_addr)
    }

    fn swap_as<T : Numerical>(&mut self, one : i64, two : i64) -> MemResult<()> {
        let one_val = self.get_at_as::<T>(one)?;
        let two_val = self.get_at_as::<T>(two)?;
        self.setmem(one, two_val)?;
        self.setmem(two, one_val)?;
        Ok(())
    }

    fn push<T : Numerical>(&mut self) -> Result<(), InvokeErr> { // get a value from somewhere in memory and push it to stack
        let loc : i64 = self.pop_arg().map_err(InvokeErr::MemErr)?;
        let val : T = self.get_at_as(loc);
        self.push(val).map_err(InvokeErr::MemErr)?;
        Ok(())
    }

    fn pushv<T : Numerical>(&mut self) -> Result<(), InvokeErr> { // push a value to the stack
        let val : T = self.pop_arg().map_err(InvokeErr::MemErr)?;
        self.push(val).map_err(InvokeErr::MemErr)?;
        Ok(())
    }

    fn swap<T : Numerical>(&mut self) -> Result<(), InvokeErr> {
        let swap_point_one = self.pop_arg::<i64>().map_err(InvokeErr::MemErr)?;
        let swap_point_two = self.pop_arg::<i64>().map_err(InvokeErr::MemErr)?;
        self.swap_as::<T>(swap_point_one, swap_point_two).map_err(InvokeErr::MemErr)?;
        Ok(())
    }

    fn pop<T : Numerical>(&mut self) -> Result<(), InvokeErr> {
        self.pop_as::<T>().map_err(InvokeErr::MemErr)?;
        Ok(())
    }

    fn popm<T : Numerical>(&mut self) -> Result<(), InvokeErr> {
        let val : T = self.pop_as().map_err(InvokeErr::MemErr)?;
        let loc : i64 = self.pop_arg().map_err(InvokeErr::MemErr)?;
        self.setmem(loc, val).map_err(InvokeErr::MemErr)?;
        Ok(())
    }

    fn cpy<T : Numerical>(&mut self) -> Result<(), InvokeErr> {
        let loc_one : i64 = self.pop_arg().map_err(InvokeErr::MemErr)?;
        let loc_two : i64 = self.pop_arg().map_err(InvokeErr::MemErr)?;
        let val : T = self.get_at_as(loc_one);
        self.setmem(loc_two, val);
        Ok(())
    }

    fn cpyv<T : Numerical>(&mut self) -> Result<(), InvokeErr> {
        let loc : i64 = self.pop_arg().map_err(InvokeErr::MemErr)?;
        let val : T = self.pop_arg().map_err(InvokeErr::MemErr)?;
        self.setmem(loc, val);
        Ok(())
    }

    fn add<T: Numerical>(&mut self) -> Result<(), InvokeErr> {
        let loc1 : i64 = self.pop_arg().map_err(InvokeErr::MemErr)?;
        let loc2 : i64 = self.pop_arg().map_err(InvokeErr::MemErr)?;
        let val1 : T = self.get_at_as(loc1).map_err(InvokeErr::MemErr)?;
        let val2 : T = self.get_at_as(loc2).map_err(InvokeErr::MemErr)?;
        let val = val1 + val2;
        self.setmem(loc1, val).map_err(InvokeErr::MemErr)?;
        Ok(())
    }

    fn sub<T: Numerical>(&mut self) -> Result<(), InvokeErr> {
        let loc1 : i64 = self.pop_arg().map_err(InvokeErr::MemErr)?;
        let loc2 : i64 = self.pop_arg().map_err(InvokeErr::MemErr)?;
        let val1 : T = self.get_at_as(loc1).map_err(InvokeErr::MemErr)?;
        let val2 : T = self.get_at_as(loc2).map_err(InvokeErr::MemErr)?;
        let val = val1 - val2;
        self.setmem(loc1, val).map_err(InvokeErr::MemErr)?;
        Ok(())
    }

    fn mul<T: Numerical>(&mut self) -> Result<(), InvokeErr> {
        let loc1 : i64 = self.pop_arg().map_err(InvokeErr::MemErr)?;
        let loc2 : i64 = self.pop_arg().map_err(InvokeErr::MemErr)?;
        let val1 : T = self.get_at_as(loc1).map_err(InvokeErr::MemErr)?;
        let val2 : T = self.get_at_as(loc2).map_err(InvokeErr::MemErr)?;
        let val = val1 * val2;
        self.setmem(loc1, val).map_err(InvokeErr::MemErr)?;
        Ok(())
    }

    fn div<T: Numerical>(&mut self) -> Result<(), InvokeErr> {
        let loc1 : i64 = self.pop_arg().map_err(InvokeErr::MemErr)?;
        let loc2 : i64 = self.pop_arg().map_err(InvokeErr::MemErr)?;
        let val1 : T = self.get_at_as(loc1).map_err(InvokeErr::MemErr)?;
        let val2 : T = self.get_at_as(loc2).map_err(InvokeErr::MemErr)?;
        let val = val1 / val2;
        self.setmem(loc1, val).map_err(InvokeErr::MemErr)?;
        Ok(())
    }

    fn cmp<T : Numerical + TryFrom<i32>>(&mut self) -> Result<(), InvokeErr> where <T as TryFrom<i32>>::Error : Debug {
        let reg : u8 = self.pop_arg().map_err(InvokeErr::MemErr)?;
        let regv : T = self.getreg_as(reg);
        if regv < 0.try_into().unwrap() { // this is infallible
            self.registers[reg as usize] = 1u64.to_be();
        }
        else {
            self.registers[reg as usize] = 0u64.to_be();
        }
        Ok(())
    }

    fn shift<T : Numerical>(&mut self) -> Result<(), InvokeErr> {
        let loc : i64 = self.pop_arg().map_err(InvokeErr::MemErr)?;
        let val : T = self.get_at_as(loc).map_err(InvokeErr::MemErr)?;
        let amount : i8 = self.pop_arg().map_err(InvokeErr::MemErr)?;
        if amount < 0 {
            self.setmem(loc, val << -amount).map_err(InvokeErr::MemErr)?;
        }
        else if amount > 0 {
            self.setmem(loc, val >> amount).map_err(InvokeErr::MemErr)?;
        }
    }

    fn throw(&mut self, code : u8) -> Result<(), InvokeErr> {
        self.errcode = code;
        if self.sbm.0 != 0 || self.sbm.1 != 0 {
            self.stack_pointer = self.sbm.0 + 16;
            self.exec_pointer = self.sbm.1;
            // doesn't remove the old sbm from stack; this must be done via checkerr.
        }
        else {
            return Err(InvokeErr::UncaughtThrow);
        }
        Ok(())
    }

    fn start_mmu(&mut self, pagesize : u32) {
        // start the builtin mmu.
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use super::invoke::*;
    use super::ir;
    #[test]
    fn abi_call() { // a simple abi call written in raw bytecode
        let image = Image {
            function_table : HashMap::from([("main".to_string(), 0i64)]),
            static_table : HashMap::new(),
            static_section : Vec::from(b"\0\0\0\0\0\0\0\0stdabi\0stest\0STDABI TEST\0"), // the 0 space is to store
                                                                            // the stdabi rabbit
            text_section : vec![68, 0, 0, 0, 0, 0, 0, 0, 8, // dock, 8: load the stdabi
                                69, 0, 0, 0, 0, 0, 0, 0, 15, // loadfun, 15: load the symbol "print" from the stdabi
                                0 , 0, 0, 0, 0, 0, 0, 0, 21, // pushvl, 21
                                67, 255, 255, 255, 255, 255, 255, 255, 240, // invokevirtual, -16
                                70] // exit
        };
        let mut machine = Machine::new(1024); // create a 1kb machine
        machine.mount(&image);
        assert_eq!(machine.invoke(image.lookup("main".to_string())), Ok(InvokeResult::StdabiTestSuccess));
    }

    #[test]
    fn ir_test() { // uses the IR compiler to run a program equivalent to above (although with an extra function call)
        let image = ir::build(r#"
=message bytes "STDABI TEST\0"
=stdabi bytes "stdabi\0"
=stest bytes "stest\0"
=stest_rabbit word 0        ; reserved space for the print function we're loading from
                            ; outside the VM
.printout
    pushvl 0                ; reserve space for the print function's argument
    movml -24 2             ; move the argument passed to this function into register 2
    movrl -8 2              ; copy the value of register 2 into the space we allocated above
    invokevirtual $stest_rabbit
    popl 2                  ; unwind the local section of the stack
    ret
.main export
    dock $stdabi
    loadfun $stest
    swapl -8 $stest_rabbit  ; shove the rabbit function in the $print_rabbit location
    pushvl $message         ; push the address of the message we're printing to stack
    call $printout
    exit 0
        "#);
        let mut machine = Machine::new(1024); // these stupid little 1kb machines are unreasonably fun
        machine.mount(&image);
        assert_eq!(machine.invoke(image.lookup("main".to_string())), Ok(InvokeResult::StdabiTestSuccess));
    }

    #[test]
    fn branch_test() {
        let image = ir::build(r#"
=test_success bytes "STDABI TEST\0"
=test_failure bytes "FAILURE\0"
=stdabi bytes "stdabi\0"
=stest bytes "stest\0"
=stest_rabbit word 0

.success
    pushvl $test_success
    invokevirtual $stest_rabbit
    exit 0

.main export
    dock $stdabi
    loadfun $stest
    swapl -8 $stest_rabbit
    movvl 1 3
    subv 3 1
    branch 3 $success
    pushvl $test_failure
    invokevirtual $stest_rabbit
    exit 0
"#);
        let mut machine = Machine::new(1024);
        machine.mount(&image);
        assert_eq!(machine.invoke(image.lookup("main".to_string())), Ok(InvokeResult::StdabiTestSuccess));
    }

    #[test]
    fn exit_value_test() {
        let image = ir::build(r#"
.main export
        exit 1234
"#);
        let mut machine = Machine::new(1024);
        machine.mount(&image);
        assert_eq!(machine.invoke(image.lookup("main".to_string())), Ok(InvokeResult::Ok(1234)));
    }

    #[test]
    fn avc_test() {
        let image = avc::build(r#"
long stdabi;
long stest;

fn getstr() -> &byte {
    "STDABI TEST"
}

fn do_print() {
    stest(getstr());
}

fn main() {
    stdabi = @load_lib("stdabi");
    stest = @load_fun(stdabi, "stest");
    do_print();
    @exit();
}
        "#);
        let mut machine = Machine::new(2048);
        machine.mount(&image);
        let output = machine.invoke(image.lookup("main".to_string()));
    }
}
