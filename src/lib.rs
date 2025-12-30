use std::pin::Pin;

use async_compression::futures::bufread::BrotliDecoder;
use futures::{AsyncRead, StreamExt, TryStreamExt, io::BufReader};
use js_sys::{ArrayBuffer, Uint8Array};
use wasm_bindgen::{JsCast, JsError, JsValue, prelude::wasm_bindgen};
use web_sys::ReadableStream;

#[wasm_bindgen(raw_module = "__wbindgen_placeholder__")]
extern "C" {
    fn __wbindgen_debug_string(js: &JsValue) -> String;
}

fn debug_string(js: JsValue) -> String {
    __wbindgen_debug_string(&js)
}
fn jsval_to_io(js: JsValue) -> std::io::Error {
    std::io::Error::other(debug_string(js))
}

fn jsval_to_vec(val: JsValue) -> Result<Vec<u8>, JsValue> {
    if let Some(str) = val.as_string() {
        Ok(str.into())
    } else if let Some(arr) = val.dyn_ref::<ArrayBuffer>() {
        Ok(Uint8Array::new(arr).to_vec())
    } else {
        Err(JsError::new("Invalid payload").into())
    }
}

fn attempt_byob(stream: ReadableStream) -> Result<Pin<Box<dyn AsyncRead>>, JsValue> {
    Ok(
        match wasm_streams::ReadableStream::from_raw(stream).try_into_async_read() {
            Ok(read) => Box::pin(read) as Pin<Box<dyn AsyncRead>>,
            Err((_, read)) => Box::pin(
                read.try_into_stream()
                    .map_err(|(err, _)| err)?
                    .map(|x| jsval_to_vec(x.map_err(jsval_to_io)?).map_err(jsval_to_io))
                    .into_async_read(),
            ) as Pin<Box<dyn AsyncRead>>,
        },
    )
}

#[wasm_bindgen]
pub fn decompress(stream: ReadableStream, buf_size: usize) -> Result<ReadableStream, JsValue> {
    let stream = attempt_byob(stream)?;

    let decompressed = BrotliDecoder::new(BufReader::with_capacity(buf_size, stream));

    Ok(wasm_streams::ReadableStream::from_async_read(decompressed, buf_size).into_raw())
}
