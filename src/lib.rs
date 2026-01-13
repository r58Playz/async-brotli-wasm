use std::pin::Pin;

use async_compression::futures::bufread::BrotliDecoder;
use futures::{AsyncRead, AsyncReadExt, StreamExt, TryStreamExt, io::BufReader, stream::unfold};
use js_sys::{ArrayBuffer, Uint8Array};
use wasm_bindgen::{JsCast, JsError, JsValue, prelude::wasm_bindgen};
use web_sys::ReadableStream;

#[wasm_bindgen(raw_module = "__wbindgen_placeholder__")]
extern "C" {
    fn __wbindgen_debug_string(js: &JsValue) -> String;
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console, js_name = log)]
    fn console_log(s: &str);
}

fn debug_string(js: JsValue) -> String {
    __wbindgen_debug_string(&js)
}
fn jsval_to_io(js: JsValue) -> std::io::Error {
    std::io::Error::other(debug_string(js))
}

fn io_to_jsval(io: std::io::Error) -> JsValue {
    format!("{}", io).into()
}

fn jsval_to_vec(val: JsValue) -> Result<Vec<u8>, JsValue> {
    if let Some(str) = val.as_string() {
        Ok(str.into())
    } else if let Some(arr) = val.dyn_ref::<ArrayBuffer>() {
        Ok(Uint8Array::new(arr).to_vec())
    } else if let Some(arr) = val.dyn_ref::<Uint8Array>() {
        Ok(arr.to_vec())
    } else {
		let mut string = __wbindgen_debug_string(&val);
		string.insert_str(0, "Invalid payload: ");
        Err(JsError::new(&string).into())
    }
}

fn attempt_byob_in(stream: ReadableStream) -> Result<(Pin<Box<dyn AsyncRead>>, bool), JsValue> {
    Ok(
        match wasm_streams::ReadableStream::from_raw(stream).try_into_async_read() {
            Ok(read) => (Box::pin(read) as Pin<Box<dyn AsyncRead>>, false),
            Err((_, read)) => (
                Box::pin(
                    read.try_into_stream()
                        .map_err(|(err, _)| err)?
                        .map(|x| jsval_to_vec(x.map_err(jsval_to_io)?).map_err(jsval_to_io))
                        .into_async_read(),
                ) as Pin<Box<dyn AsyncRead>>,
                true,
            ),
        },
    )
}

fn async_read_to_stream(
    stream: impl AsyncRead + Unpin + 'static,
    buf_size: usize,
) -> ReadableStream {
    let buf = vec![0; buf_size];
    let reader = unfold((stream, buf), |(mut reader, mut buf)| async {
        match reader.read(&mut buf).await {
            Ok(0) => None,
            Ok(n) => Some((
                Ok(Uint8Array::new_from_slice(&buf[..n]).into()),
                (reader, buf),
            )),
            Err(err) => Some((Err(io_to_jsval(err)), (reader, buf))),
        }
    });

    wasm_streams::ReadableStream::from_stream(reader).into_raw()
}

#[wasm_bindgen]
pub fn decompress(stream: ReadableStream, buf_size: usize) -> Result<ReadableStream, JsValue> {
    let (stream, failed_byob) = attempt_byob_in(stream)?;

    let decompressed = BrotliDecoder::new(BufReader::with_capacity(buf_size, stream));

    if failed_byob {
		console_log("async_brotli_wasm: failed to use byob streams. falling back to slower copying");
		Ok(async_read_to_stream(decompressed, buf_size))
    } else {
        Ok(wasm_streams::ReadableStream::from_async_read(decompressed, buf_size).into_raw())
    }
}
