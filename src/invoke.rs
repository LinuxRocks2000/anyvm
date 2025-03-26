use crate::error::*;
use crate::{ Machine, RabbitTable, AbiFunction };
use std::ffi::CStr;
use std::collections::HashMap;


impl Machine {
    pub fn invoke(&mut self, at : i64) -> Result<InvokeResult, InvokeErr> { // set up the stack and loop through operations until exit() is called
        self.exec_pointer = at as u64;
        self.stack_pointer = self.stack_start as u64;
        loop {
            let op = self.pop_arg::<u8>().map_err(InvokeErr::MemErr)?;
            let old_errcode = self.errcode;
            self.errcode = 0;
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
                54 => { // bor
                    let loc1 = self.pop_arg::<i64>().map_err(InvokeErr::MemErr)?;
                    let val1 = self.get_at_as::<u8>(loc1).map_err(InvokeErr::MemErr)?;
                    let loc2 = self.pop_arg::<i64>().map_err(InvokeErr::MemErr)?;
                    let val2 = self.get_at_as::<u8>(loc2).map_err(InvokeErr::MemErr)?;
                    self.setmem(loc1, val1 | val2).map_err(InvokeErr::MemErr)?;
                },
                55 => { // vor
                    let loc1 = self.pop_arg::<i64>().map_err(InvokeErr::MemErr)?;
                    let val1 = self.get_at_as::<u8>(loc1).map_err(InvokeErr::MemErr)?;
                    let val2 = self.pop_arg::<u8>().map_err(InvokeErr::MemErr)?;
                    self.setmem(loc1, val1 | val2).map_err(InvokeErr::MemErr)?;
                },
                56 => { // band
                    let loc1 = self.pop_arg::<i64>().map_err(InvokeErr::MemErr)?;
                    let val1 = self.get_at_as::<u8>(loc1).map_err(InvokeErr::MemErr)?;
                    let loc2 = self.pop_arg::<i64>().map_err(InvokeErr::MemErr)?;
                    let val2 = self.get_at_as::<u8>(loc2).map_err(InvokeErr::MemErr)?;
                    self.setmem(loc1, val1 & val2).map_err(InvokeErr::MemErr)?;
                },
                57 => { // vand
                    let loc1 = self.pop_arg::<i64>().map_err(InvokeErr::MemErr)?;
                    let val1 = self.get_at_as::<u8>(loc1).map_err(InvokeErr::MemErr)?;
                    let val2 = self.pop_arg::<u8>().map_err(InvokeErr::MemErr)?;
                    self.setmem(loc1, val1 & val2).map_err(InvokeErr::MemErr)?;
                },
                // shift[l, i, s, b]
                58 => { self.shift::<u64>()?; },
                59 => { self.shift::<u32>()?; },
                60 => { self.shift::<u16>()?; },
                61 => { self.shift::<u8>()?; },
                62 => { // bnorm
                    let loc = self.pop_arg::<i64>().map_err(InvokeErr::MemErr)?;
                    let val : u8 = self.get_at_as(loc).map_err(InvokeErr::MemErr)?;
                    self.setmem::<u8>(loc, if val == 0 { 0 } else { 1 });
                },
                63 => { // jmp
                    let amnt : i64 = self.pop_arg();
                    self.exec_pointer += amnt;
                },

                // flow control
                64 => { // branch
                    let val = self.pop_arg::<u8>().map_err(InvokeErr::MemErr)?;
                    if val == 0 {
                        self.exec_pointer = pos;
                    }
                },
                65 => { // call
                    let addr = self.pop_arg::<u64>().map_err(InvokeErr::MemErr)?;
                    self.push(self.exec_pointer).map_err(InvokeErr::MemErr)?; // push the return address.
                    // the stack frame should now look like [return value space] [arguments] [return address].
                    // the first thing the called function should do upon being invoked is increment the stack
                    // so it looks like [return value space] [arguments] [return address] [locals]
                    self.exec_pointer = addr;
                },
                66 => { // ret
                    // the called function should have already decremented the stack so [return address]
                    // is the highest value on it.
                    let ret_addr = self.pop_as::<u64>().map_err(InvokeErr::MemErr)?;
                    self.exec_pointer = ret_addr;
                },
                67 => { // invokevirtual
                    let loc : i64 = self.pop_arg().map_err(InvokeErr::MemErr)?;
                    let place : i64 = self.get_at_as(loc).map_err(InvokeErr:MemErr)?;
                    self.push(self.exec_pointer).map_err(InvokeErr::MemErr)?;
                    self.exec_pointer = place;
                },
                68 => {
                    // TODO: invokeext
                    // grab a function id from memory,
                    // check if that function id is mapped into the current machine,
                    // if it is, setsbm and invoke that function
                    // if it isn't, throw.
                },
                69 => { // setsbm
                    self.push(self.sbm.0).map_err(InvokeErr::MemErr)?;
                    self.push(self.sbm.1).map_err(InvokeErr::MemErr)?;
                    self.sbm = (self.stack_pointer, self.exec_pointer + 9);
                },
                70 => { // throw
                    let code : u8 = self.pop_arg().map_err(InvokeErr::MemErr)?;
                    self.throw(code)?;
                },
                71 => { // checkerr
                    let target : i64 = self.pop_arg();
                    if old_errcode != 0 {
                        self.errcode = old_errcode;
                        self.exec_pointer = target;
                    }
                    self.sbm.1 = self.pop_as(); // pop sbm off stack
                    self.sbm.0 = self.pop_as();
                },
                72 => { // geterr
                    self.push_as(old_errcode);
                }
                73 => { // exit
                    let out = self.pop_arg::<i64>().map_err(InvokeErr::MemErr)?;
                    return Ok(InvokeResult::Ok(out));
                },
                74 => {
                    let pagesize = self.pop_arg::<u32>().map_err(InvokeErr::MemErr);
                    self.start_mmu(pagesize);
                },
                _ => {
                    return Err(InvokeErr::BadInstruction);
                }
            }
        }
        Ok(InvokeResult::Ok(0))
    }
}
