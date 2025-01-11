use crate::error::*;
use crate::{ Machine, RabbitTable, AbiFunction };
use std::ffi::CStr;
use std::collections::HashMap;


impl Machine {
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
                
                56 => { // not
                    let reg = self.pop_arg::<u8>().map_err(InvokeErr::MemErr)?;
                    if self.getreg_as::<u64>(reg) == 0 {
                        self.registers[reg as usize] = 1;
                    }
                    else {
                        self.registers[reg as usize] = 0;
                    }
                },
                57 => { // or
                    let reg1 = self.pop_arg::<u8>().map_err(InvokeErr::MemErr)?;
                    let reg2 = self.pop_arg::<u8>().map_err(InvokeErr::MemErr)?;
                    self.registers[reg1 as usize] = self.getreg_as::<u64>(reg1) | self.getreg_as::<u64>(reg2);
                },
                58 => { // and
                    let reg1 = self.pop_arg::<u8>().map_err(InvokeErr::MemErr)?;
                    let reg2 = self.pop_arg::<u8>().map_err(InvokeErr::MemErr)?;
                    self.registers[reg1 as usize] = self.getreg_as::<u64>(reg1) & self.getreg_as::<u64>(reg2);
                },
                59 => { // or
                    let reg1 = self.pop_arg::<u8>().map_err(InvokeErr::MemErr)?;
                    let reg2 = self.pop_arg::<u8>().map_err(InvokeErr::MemErr)?;
                    self.registers[reg1 as usize] = self.getreg_as::<u64>(reg1) ^ self.getreg_as::<u64>(reg2);
                },

                // flow control
                64 => { // branch
                    let reg = self.pop_arg::<u8>().map_err(InvokeErr::MemErr)?;
                    let pos = self.pop_arg::<u64>().map_err(InvokeErr::MemErr)?;
                    if self.registers[reg as usize] == 0 {
                        self.registers[0] = pos;
                    }
                },
                65 => { // call
                    let addr = self.pop_arg::<u64>().map_err(InvokeErr::MemErr)?;
                    self.push(self.registers[0] + 1).map_err(InvokeErr::MemErr)?; // push the return address.
                    // the stack frame should now look like [return value space] [arguments] [return address].
                    // the first thing the called function should do upon being invoked is increment the stack
                    // so it looks like [return value space] [arguments] [return address] [locals]
                    self.registers[0] = addr;
                },
                66 => { // ret
                    // the called function should have already decremented the stack so [return address]
                    // is the highest value on it.
                    let ret_addr = self.pop_as::<u64>().map_err(InvokeErr::MemErr)?;
                    self.registers[0] = ret_addr;
                },
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