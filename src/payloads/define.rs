pub type PAYLOADFUNC = PayloadFunc;
pub struct PayloadFunc {
    pub times: i32,
    pub runtime: i32,
}

pub type PAYLOAD = Payload;
type PayloadFn = fn(i32, i32) -> i32;
pub struct Payload {
    pub payload_function: PayloadFn,
    pub delay: i32,
}

