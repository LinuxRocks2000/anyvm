// error handling stuff


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


pub fn str_proc_fail<T>(_ : T) -> InvokeErr {
    InvokeErr::StringProcessingError
}


pub type MemResult<T> = Result<T, MemoryErr>;