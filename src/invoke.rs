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
                0 => { self.push::<u64>()?; }, // why, do you ask, did I choose this pattern?
                1 => { self.push::<u32>()?; }, // you don't want to know.
                2 => { self.push::<u16>()?; }, // useful for documentation purposes?
                3 => { self.push::<u8>()?; },  // no. screw off. pretend I didn't do it this way.
                // push[l, i, s, b]
                4 => { self.pushv::<u64>()?; },
                5 => { self.pushv::<u32>()?; },
                6 => { self.pushv::<u16>()?; },
                7 => { self.pushv::<u8>()?; },
                // swap[l, i, s, b]
                8 => { self.swap::<u64>()?; },
                9 => { self.swap::<u32>()?; },
                10 => { self.swap::<u16>()?; },
                11 => { self.swap::<u8>()?; },
                // cpy[l, i, s, b]
                12 => { self.cpy::<u64>()?; },
                13 => { self.cpy::<u32>()?; },
                14 => { self.cpy::<u16>()?; },
                15 => { self.cpy::<u8>()?; },
                // cpyv[l, i, s, b]
                16 => { self.cpyv::<u64>()?; },
                17 => { self.cpyv::<u32>()?; },
                18 => { self.cpyv::<u16>()?; },
                19 => { self.cpyv::<u8>()?; },
                // pop[l, i, s, b]
                20 => { self.pop::<u64>()?; },
                21 => { self.pop::<u32>()?; },
                22 => { self.pop::<u16>()?; },
                23 => { self.pop::<u8>()?; },
                // popm[l, i, s, b]
                24 => { self.popm::<u64>()?; },
                25 => { self.popm::<u32>()?; },
                26 => { self.popm::<u16>()?; },
                27 => { self.popm::<u8>()?; },
                
                // arithmetic
                // add
                28 => { self.add::<u64>()?; },
                29 => { self.add::<u32>()?; },
                30 => { self.add::<u16>()?; },
                31 => { self.add::<u8>()?; },

                // sub
                32 => { self.sub::<u64>()?; },
                33 => { self.sub::<u32>()?; },
                34 => { self.sub::<u16>()?; },
                35 => { self.sub::<u8>()?; },

                // mul
                36 => { self.mul::<u64>()?; },
                37 => { self.mul::<u32>()?; },
                38 => { self.mul::<u16>()?; },
                39 => { self.mul::<u8>()?; },

                // div
                40 => { self.div::<u64>()?; },
                41 => { self.div::<u32>()?; },
                42 => { self.div::<u16>()?; },
                43 => { self.div::<u8>()?; },

                // logical operations
                
                // cmp[l, i, s, b]
                44 => { self.cmp::<u64>()?; },
                45 => { self.cmp::<u32>()?; },
                46 => { self.cmp::<u16>()?; },
                47 => { self.cmp::<u8>()?; },
                
                // cmpv[l, i, s, b]
                48 => { self.cmpv::<u64>()?; },
                49 => { self.cmpv::<u32>()?; },
                50 => { self.cmpv::<u16>()?; },
                51 => { self.cmpv::<u8>()?; },
                
                52 => { // bnot
                    let loc = self.pop_arg::<i64>().map_err(InvokeErr::MemErr)?;
                    let val = self.get_at_as::<u8>(loc).map_err(InvokeErr::MemErr)?;
                    self.setmem(loc, !val).map_err(InvokeErr::MemErr)?;
                    Ok(())
                },
                53 => { // not
                    let loc = self.pop_arg::<i64>().map_err(InvokeErr::MemErr)?;
                    let val = self.get_at_as::<u8>(loc).map_err(InvokeErr::MemErr)?;
                    self.setmem(loc, if val == 0 { 1 } else { 0 }).map_err(InvokeErr::MemErr)?;
                    Ok(())
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
                    self.push(self.registers[0]).map_err(InvokeErr::MemErr)?; // push the return address.
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
                    // TODO: make this actually call virtual functions [right now it only calls rabbit functions]
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