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
    // basic memory operations (pushing, popping, swapping, moving)
    // note that many of these are generic over architecture-supported sizes: l = 64 bit, i = 32 bit, s = 16 bit, b = 8 bit.

    0 -> 3. pushv[l, i, s, b]: push a value to the stack.
    4 -> 7. swap[l, i, s, b]: swap two values in memory
    8 -> 11. pop[l, i, s, b]: pop the top value from stack and store it in a register
    12 -> 15. movv[l, i, s, b]: move a static value to a register.
    16 -> 19. movm[l, i, s, b]: move a value from memory to a register.
    20 -> 23. movr[l, i, s, b]: move a value from a register to memory (the high bits will be truncated for movri, movrs, and movrb)

    // int arithmetic
    // all arithmetic is performed in the 64-bit registers. the implementations are generic across signedness and size, so no conversions are necessary.
    // arithmetic operations can be normal or v. v means that, rather than a second register, the value to use is static, like movv.

    24 -> 25. add(v): add
    26 -> 27. sub(v): subtract
    28 -> 29. mul(v): multiply
    30 -> 31. div(v): divide   // note: I haven't yet checked if bitwise division actually does work the same for signed and unsigned values. oops.
    // may need to expand this to div, divv, idiv, and idivv to include signedness

    // logical bitwise operations
    40 -> 43. cmplt[l, i, s, b]: if the value of the specified register is less than 0, set the value of the register to 1u64; otherwise, set it to 0
    44 -> 47. cmpgt[l, i, s, b]: same as cmplt, except for *greater than* 0
    48 -> 51. cmplte[l, i, s, b]: less than or equal to 0
    52 -> 55. cmpgte[l, i, s, b]: greater than or equal to 0
    56. not: if the value of a register is equal to 0u64, set it to 1u64; else, set it to 0u64
    57. or: set the value of the first register to the bitwise OR of the specified registers
    58. and: see or, but and
    59. xor: see or, but xor

    // flow control
    64. branch: if the value in a specified register is 0, branch to a specified location. else, continue with the next operation.
    65. call: call a function.
        there is no JMP instruction. this is because directly modifying the value of register 0 with addv is valid.
        call is preferable because it saves several instructions by pushing the correct return address to the stack for you.
        you have to push the arguments to stack *before* `call`ing, and the function must still handle stack allocating its own local variables.
        the first stack push a caller makes should be reserving space for the function's return value, if any.
    66. ret: return from a function. expects the top value on the stack to be the return address - that is, the callee function has to unwind the stack down to the return address
        before calling ret. ret is equivalent to popl, 0.
    67. invokevirtual: `call`, except it dereferences the argument to a 64-bit value somewhere in memory and performs rabbit checks.
        where call, <rabbit> and movl, <rabbit>, 0 will both fail by out-of-bounds, `invokevirtual` will perform a lookup in the rabbit table and call abi functions or
        cross-VM functions. the extra performance overhead makes it less than ideal for function calls that don't *have* to be virtual.

    // vm commands
    // most of these should be treated as blackboxy.
    68. dock: load something from outside. this creates a magic table at a rabbit address. the argument must be a null-terminated ascii string describing the thing we're docking.
        the rabbit address is pushed to stack, and should be swapl'd into a static location.
    69. loadfun: load a function from a magic table stored in a rabbit address. the name of the function must be a null-terminated ascii string. the rabbit address should be at the top of the stack.
        this will push the rabbit address of the function to stack.
    70. exit: exit the VM

    there are some gaps. these are reserved; eventually, they may contain floating-point instructions. As yet there is no floating-point support in anyvm.
*/
use std::collections::HashMap;
use std::ffi::CStr;

mod numerical;
use numerical::*;


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


pub trait BoxCloneInternal {
    fn clone_box(&self) -> Box<dyn MagicallyCallable>;
}


impl<T> BoxCloneInternal for T where T : MagicallyCallable + Clone + 'static {
    fn clone_box(&self) -> Box<dyn MagicallyCallable> {
        Box::new(self.clone())
    }
}


impl Clone for Box<dyn MagicallyCallable> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}


pub trait MagicallyCallable : BoxCloneInternal {
    fn call(&self, machine : &mut Machine) -> Result<InvokeResult, InvokeErr>;
}


#[derive(Clone)]
pub struct AbiFunction {
    function : fn(&mut Machine) -> Result<InvokeResult, InvokeErr>
}


impl AbiFunction {
    pub fn new(function : fn(&mut Machine) -> Result<InvokeResult, InvokeErr>) -> Box<dyn MagicallyCallable> {
        Box::new(Self {
            function
        })
    }
}


impl MagicallyCallable for AbiFunction {
    fn call(&self, machine : &mut Machine) -> Result<InvokeResult, InvokeErr> {
        (self.function)(machine)
    }
}


struct RabbitTable {
    fns : HashMap<String, Box<dyn MagicallyCallable>>
}


pub struct Machine {
    memory : Vec<u8>,
    registers : [u64; 256], // register 0 is op pointer, register 1 is stack pointer
    text_start : i64,
    stack_start : i64,
    end : i64,
    rabbit_top : i64,
    rabbit_objs : HashMap<i64, RabbitTable>, // essentially magical symbol tables
    rabbit_fns : HashMap<i64, Box<dyn MagicallyCallable>> // functions that can be called out into
}


#[derive(Debug, PartialEq)]
pub enum InvokeResult {
    Ok,
    StdabiTestSuccess
}


#[derive(Debug, PartialEq)]
pub enum MemoryErr { // errors specifically related to memory
    OutOfMemory,
    SegmentationFault // thrown if you try to do accesses below 0 or beyond the vm memory (rabbit addresses cannot be manipulated by most operations)
}


#[derive(Debug, PartialEq)]
pub enum InvokeErr {
    MemErr(MemoryErr),
    BadInstruction,
    StdabiTestFailure,
    StringProcessingError // failed to build a null-terminated CStr
}


fn str_proc_fail<T>(_ : T) -> InvokeErr {
    InvokeErr::StringProcessingError
}


type MemResult<T> = Result<T, MemoryErr>;


impl Machine {
    pub fn new(capacity : usize) -> Machine {
        Machine {
            memory : vec![0u8; capacity],
            registers : [0u64; 256], // 2kb of register space. plenty!
                                     // on the lil' 1kb machines I'm so fond of, there is actually twice as much
                                     // space in the register block than there is in the VM memory
            end : capacity as i64 - 8, // 8 byte padding at the end. why? to save a tonne of cycles. more below.
            stack_start : 0,
            text_start : 0,
            rabbit_top : capacity as i64 + 1,
            rabbit_fns : HashMap::new(),
            rabbit_objs : HashMap::new()
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
            addr += self.registers[1] as i64
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

    fn pop_arg<T : Numerical>(&mut self) -> MemResult<T> { // pop an arg
        let ret = unsafe { self.memory_as_at::<T>(self.registers[0] as usize)?[0] };
        self.registers[0] += T::BYTE_COUNT as u64;
        Ok(ret.from_be())
    }

    fn pop_arg_addr(&mut self) -> MemResult<usize> { // pop an argument and convert it to a stackaddr
        let arg = self.pop_arg::<i64>()?;
        self.stackaddr(arg)
    }

    fn pop_as<T : Numerical>(&mut self) -> MemResult<T> { // pop a thing off stack
        let r = self.get_at_as::<T>(-(T::BYTE_COUNT as i64));
        self.registers[1] -= T::BYTE_COUNT as u64;
        r
    }

    fn push<T : Numerical>(&mut self, thing : T) -> MemResult<()> { // push a thing to stack
        unsafe {
            self.memory_as_at::<T>(self.registers[1] as usize)?[0] = thing.to_be();
        }
        self.registers[1] += T::BYTE_COUNT as u64;
        Ok(())
    }

    fn pop_addr(&mut self) -> MemResult<usize> { // pop an address off stack and run it through stackaddr()
        let vm_addr = self.pop_as::<i64>()?;
        self.stackaddr(vm_addr)
    }

    fn swap_as<T : Numerical>(&mut self, one : i64, two : i64) -> MemResult<()> {
        let one_val = self.get_at_as::<T>(one)?;
        let two_val = self.get_at_as::<T>(two)?;
        unsafe {
            self.memory_as_at::<T>(self.stackaddr(one)?)?[0] = two_val;
            self.memory_as_at::<T>(self.stackaddr(two)?)?[0] = one_val;
        }
        Ok(())
    }

    fn pusher<T : Numerical>(&mut self) -> Result<(), InvokeErr> {
        let val : T = self.pop_arg().map_err(InvokeErr::MemErr)?;
        self.push(val).map_err(InvokeErr::MemErr)?;
        Ok(())
    }

    fn swapper<T : Numerical>(&mut self) -> Result<(), InvokeErr> {
        let swap_point_one = self.pop_arg::<i64>().map_err(InvokeErr::MemErr)?;
        let swap_point_two = self.pop_arg::<i64>().map_err(InvokeErr::MemErr)?;
        self.swap_as::<T>(swap_point_one, swap_point_two).map_err(InvokeErr::MemErr)?;
        Ok(())
    }

    fn popper<T : Numerical>(&mut self) -> Result<(), InvokeErr> {
        let val : T = self.pop_as().map_err(InvokeErr::MemErr)?;
        let register : u8 = self.pop_arg().map_err(InvokeErr::MemErr)?;
        self.registers[register as usize] = val.naive_u64();
        Ok(())
    }

    fn movver<T : Numerical>(&mut self) -> Result<(), InvokeErr> {
        let val : T = self.pop_arg().map_err(InvokeErr::MemErr)?;
        let register : u8 = self.pop_arg().map_err(InvokeErr::MemErr)?;
        self.registers[register as usize] = val.naive_u64();
        Ok(())
    }

    fn movmer<T : Numerical>(&mut self) -> Result<(), InvokeErr> {
        let addr : i64 = self.pop_arg().map_err(InvokeErr::MemErr)?;
        let val : T = self.get_at_as(addr).map_err(InvokeErr::MemErr)?;
        let register : u8 = self.pop_arg().map_err(InvokeErr::MemErr)?;
        self.registers[register as usize] = val.naive_u64();
        Ok(())
    }

    fn movrer<T : Numerical>(&mut self) -> Result<(), InvokeErr> {
        let addr = self.pop_arg_addr().map_err(InvokeErr::MemErr)?;
        let register : u8 = self.pop_arg().map_err(InvokeErr::MemErr)?;
        let rbytes = self.registers[register as usize].to_be_bytes();
        for i in 0..T::BYTE_COUNT {
            self.memory[addr + i] = rbytes[i + 7 - T::BYTE_COUNT];
        }
        Ok(())
    }

    pub fn invoke(&mut self, at : i64) -> Result<InvokeResult, InvokeErr> { // set up the stack and loop through operations until exit() is called
        self.registers[0] = at as u64;
        self.registers[1] = self.stack_start as u64;
        loop {
            let op = self.pop_arg::<u8>().map_err(InvokeErr::MemErr)?;
            match op {
                // pushv[l, i, s, b]
                0 => { self.pusher::<u64>()?; }, // why, do you ask, did I choose this pattern?
                1 => { self.pusher::<u32>()?; }, // you don't want to know.
                2 => { self.pusher::<u16>()?; }, // useful for documentation purposes?
                3 => { self.pusher::<u8>()?; },  // no. screw off. pretend I didn't do it this way.
                // swap[l, i, s, b]
                4 => { self.swapper::<u64>()?; },
                5 => { self.swapper::<u32>()?; },
                6 => { self.swapper::<u16>()?; },
                7 => { self.swapper::<u8>()?; },
                // pop[l, i, s, b]
                8 => { self.popper::<u64>()?; },
                9 => { self.popper::<u32>()?; },
                10 => { self.popper::<u16>()?; },
                11 => { self.popper::<u8>()?; },
                // movv[l, i, s, b]
                12 => { self.movver::<u64>()?; },
                13 => { self.movver::<u32>()?; },
                14 => { self.movver::<u16>()?; },
                15 => { self.movver::<u8>()?; },
                // movm[l, i, s, b]
                16 => { self.movmer::<u64>()?; },
                17 => { self.movmer::<u32>()?; },
                18 => { self.movmer::<u16>()?; },
                19 => { self.movmer::<u8>()?; },
                // movr[l, i, s, b]
                20 => { self.movrer::<u64>()?; },
                21 => { self.movrer::<u32>()?; },
                22 => { self.movrer::<u16>()?; },
                23 => { self.movrer::<u8>()?; },
                
                // arithmetic
                24 => { // add
                    let reg1 : usize = self.pop_arg::<u8>().map_err(InvokeErr::MemErr)? as usize;
                    let reg2 : usize = self.pop_arg::<u8>().map_err(InvokeErr::MemErr)? as usize;
                    self.registers[reg1] = self.registers[reg1] + self.registers[reg2];
                },
                25 => { // addv
                    let reg1 : usize = self.pop_arg::<u8>().map_err(InvokeErr::MemErr)? as usize;
                    let val : u64 = self.pop_arg().map_err(InvokeErr::MemErr)?;
                    self.registers[reg1] = self.registers[reg1] + val;
                },
                26 => { // sub
                    let reg1 : usize = self.pop_arg::<u8>().map_err(InvokeErr::MemErr)? as usize;
                    let reg2 : usize = self.pop_arg::<u8>().map_err(InvokeErr::MemErr)? as usize;
                    self.registers[reg1] = self.registers[reg1] - self.registers[reg2];
                },
                27 => { // subv
                    let reg1 : usize = self.pop_arg::<u8>().map_err(InvokeErr::MemErr)? as usize;
                    let val : u64 = self.pop_arg().map_err(InvokeErr::MemErr)?;
                    self.registers[reg1] = self.registers[reg1] - val;
                },
                28 => { // mul
                    let reg1 : usize = self.pop_arg::<u8>().map_err(InvokeErr::MemErr)? as usize;
                    let reg2 : usize = self.pop_arg::<u8>().map_err(InvokeErr::MemErr)? as usize;
                    self.registers[reg1] = self.registers[reg1] * self.registers[reg2];
                },
                29 => { // mulv
                    let reg1 : usize = self.pop_arg::<u8>().map_err(InvokeErr::MemErr)? as usize;
                    let val : u64 = self.pop_arg().map_err(InvokeErr::MemErr)?;
                    self.registers[reg1] = self.registers[reg1] * val;
                },
                30 => { // div
                    let reg1 : usize = self.pop_arg::<u8>().map_err(InvokeErr::MemErr)? as usize;
                    let reg2 : usize = self.pop_arg::<u8>().map_err(InvokeErr::MemErr)? as usize;
                    self.registers[reg1] = self.registers[reg1] * self.registers[reg2];
                },
                31 => { // divv
                    let reg1 : usize = self.pop_arg::<u8>().map_err(InvokeErr::MemErr)? as usize;
                    let val : u64 = self.pop_arg().map_err(InvokeErr::MemErr)?;
                    self.registers[reg1] = self.registers[reg1] / val;
                },

                // logical operations
                // cmplt[l, i, s, b]
                40 => { self.cmplter::<u64>()?; },
                41 => { self.cmplter::<u32>()?; },
                42 => { self.cmplter::<u16>()?; },
                43 => { self.cmplter::<u8>()?; },

                // cmpgt[l, i, s, b]
                44 => { self.cmpgter::<u64>()?; },
                45 => { self.cmpgter::<u32>()?; },
                46 => { self.cmpgter::<u16>()?; },
                47 => { self.cmpgter::<u8>()?; },

                // cmplte[l, i, s, b]
                48 => { self.cmplteer::<u64>()?; },
                49 => { self.cmplteer::<u32>()?; },
                50 => { self.cmplteer::<u16>()?; },
                51 => { self.cmplteer::<u8>()?; },

                // cmplte[l, i, s, b]
                52 => { self.cmpgteer::<u64>()?; },
                53 => { self.cmpgteer::<u32>()?; },
                54 => { self.cmpgteer::<u16>()?; },
                55 => { self.cmpgteer::<u8>()?; },

                67 => { // invokevirtual
                    let to_invoke = self.pop_arg().map_err(InvokeErr::MemErr)?;
                    let rabbit = self.get_at_as::<i64>(to_invoke).map_err(InvokeErr::MemErr)?;
                    let res = self.rabbit_fns[&rabbit].clone().call(self); // TODO: fix so we don't have to clone here [bad!]
                    match res {
                        Ok(InvokeResult::StdabiTestSuccess) | Err(_) => { return res; }, // if the abi call reports a successful test or an error, we want to exit now
                        // if it wishes for the error to be accessible *inside* the vm, it'll use the internal stack like a good citizen
                        Ok(_) => {}
                    }
                },
                68 => { // dock
                    let d_name_loc = self.pop_arg_addr().map_err(InvokeErr::MemErr)?;
                    let d_name = CStr::from_bytes_until_nul(&self.memory[d_name_loc..]).map_err(str_proc_fail)?; // TODO: error handling
                    if d_name.to_str().map_err(str_proc_fail)? == "stdabi" {
                        let rabbit = self.next_rabbit();
                        self.push(rabbit).map_err(InvokeErr::MemErr)?;
                        self.rabbit_objs.insert(rabbit, RabbitTable {
                            fns : HashMap::from([
                                    (
                                        "print".to_string(),
                                        AbiFunction::new(|machine| {
                                            let addr = machine.pop_addr().map_err(InvokeErr::MemErr)?;
                                            machine.registers[1] += 8; // restore the popped address; the caller will want to pop off arguments itself
                                            let string = CStr::from_bytes_until_nul(&machine.memory[addr..]).map_err(str_proc_fail)?.to_str().map_err(str_proc_fail)?.to_string();
                                            print!("{}", string);
                                            Ok(InvokeResult::Ok)
                                        })
                                    ),
                                    (
                                        "stest".to_string(),
                                        AbiFunction::new(|machine| {
                                            let addr = machine.pop_addr().map_err(InvokeErr::MemErr)?;
                                            machine.registers[1] += 8; // restore the popped address; the caller will want to pop off arguments itself
                                            let string = CStr::from_bytes_until_nul(&machine.memory[addr..]).map_err(str_proc_fail)?.to_str().map_err(str_proc_fail)?.to_string();
                                            if string == "STDABI TEST" {
                                                return Ok(InvokeResult::StdabiTestSuccess);
                                            }
                                            else {
                                                return Err(InvokeErr::StdabiTestFailure);
                                            }
                                        })
                                    )
                                ]
                            )
                        });
                    }
                },
                69 => { // load a function
                    let root_rabbit = self.pop_as::<i64>().map_err(InvokeErr::MemErr)?;
                    let d_name_ptr = self.pop_arg_addr().map_err(InvokeErr::MemErr)?;
                    let d_name = CStr::from_bytes_until_nul(&self.memory[d_name_ptr..]).map_err(str_proc_fail)?.to_str().map_err(str_proc_fail)?.to_string();
                    let r = self.next_rabbit();
                    self.rabbit_fns.insert(r, self.rabbit_objs[&root_rabbit].fns[&d_name].clone());
                    self.push(r).map_err(InvokeErr::MemErr)?;
                },
                70 => {
                    break;
                },
                _ => {
                    return Err(InvokeErr::BadInstruction);
                }
            }
        }
        Ok(InvokeResult::Ok)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn abi_call() {
        let image = Image {
            function_table : HashMap::from([("main".to_string(), 0i64)]),
            static_table : HashMap::new(),
            static_section : Vec::from(b"\0\0\0\0\0\0\0\0stdabi\0stest\0STDABI TEST\0"), // the 0 space is to store
                                                                            // the stdabi rabbit
            text_section : vec![68, 0, 0, 0, 0, 0, 0, 0, 8, // dock, 8: load the stdabi
                                69, 0, 0, 0, 0, 0, 0, 0, 15, // loadfun, 15: load the symbol "print" from the stdabi
                                0 , 0, 0, 0, 0, 0, 0, 0, 21, // pushvi, 21
                                67, 255, 255, 255, 255, 255, 255, 255, 240, // invokevirtual, -16
                                70] // exit
        };
        let mut machine = Machine::new(1024); // create a 1kb machine
        machine.mount(&image);
        assert_eq!(machine.invoke(image.lookup("main".to_string())), Ok(InvokeResult::StdabiTestSuccess));
    }

    #[test]
    fn function_call() {
        let image = Image {
            function_table : HashMap::from([("main".to_string(), 0i64)]),
            static_table : HashMap::new(),
            static_section : Vec::from(b"hello, world\n"),
            text_section : vec![]
        };
        let mut machine = Machine::new(1024); // these stupid little 1kb machines are unreasonably fun
        machine.mount(&image);
        assert_eq!(machine.invoke(image.lookup("main".to_string())), Ok(InvokeResult::Ok));
    }
}