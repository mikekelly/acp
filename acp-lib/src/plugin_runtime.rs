//! JavaScript plugin runtime using Boa engine
//!
//! Provides a sandboxed JavaScript environment for executing plugin transforms.
//! Implements:
//! - ACP.crypto globals (sha256, sha256Hex, hmac)
//! - ACP.util globals (base64, hex, now, isoDate, amzDate)
//! - TextEncoder/TextDecoder
//! - Sandbox restrictions (no fetch, eval, etc.)

use crate::{AcpError, Result};
use base64::Engine;
use boa_engine::{Context, JsArgs, JsNativeError, JsResult, JsString, JsValue, NativeFunction, Source};
use chrono::Utc;
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

/// JavaScript runtime for plugin execution
pub struct PluginRuntime {
    context: Context,
}

impl PluginRuntime {
    /// Create a new plugin runtime with sandboxed global objects
    pub fn new() -> Result<Self> {
        let mut context = Context::default();

        // Set up ACP global object
        Self::setup_acp_globals(&mut context)?;

        // Set up TextEncoder/TextDecoder
        Self::setup_text_encoding(&mut context)?;

        // Apply sandbox restrictions
        Self::apply_sandbox(&mut context)?;

        Ok(Self { context })
    }

    /// Execute JavaScript code in the runtime
    pub fn execute(&mut self, code: &str) -> Result<JsValue> {
        self.context
            .eval(Source::from_bytes(code))
            .map_err(|e| AcpError::plugin(format!("JavaScript execution error: {}", e)))
    }

    /// Set up ACP.crypto and ACP.util global objects
    fn setup_acp_globals(context: &mut Context) -> Result<()> {
        // Register native functions first
        Self::register_crypto_natives(context)?;
        Self::register_util_natives(context)?;

        // Create ACP namespace with crypto and util methods
        let setup_code = r#"
        var ACP = {
            crypto: {
                sha256: function(data) {
                    return __acp_native_sha256(data);
                },
                sha256Hex: function(data) {
                    return __acp_native_sha256_hex(data);
                },
                hmac: function(key, data, encoding) {
                    return __acp_native_hmac(key, data, encoding || 'hex');
                }
            },
            util: {
                base64: function(data, decode) {
                    if (decode) {
                        return __acp_native_base64_decode(data);
                    }
                    return __acp_native_base64_encode(data);
                },
                hex: function(data, decode) {
                    if (decode) {
                        return __acp_native_hex_decode(data);
                    }
                    return __acp_native_hex_encode(data);
                },
                now: function() {
                    return __acp_native_now();
                },
                isoDate: function(timestamp) {
                    return __acp_native_iso_date(timestamp);
                },
                amzDate: function(timestamp) {
                    return __acp_native_amz_date(timestamp);
                }
            }
        };
        "#;

        context.eval(Source::from_bytes(setup_code))
            .map_err(|e| AcpError::plugin(format!("Failed to create ACP namespace: {}", e)))?;

        Ok(())
    }

    /// Register native crypto functions
    fn register_crypto_natives(context: &mut Context) -> Result<()> {
        // sha256 - returns array of bytes
        let sha256_fn = NativeFunction::from_fn_ptr(|_, args, context| {
            let data = args.get_or_undefined(0);
            let bytes = js_value_to_bytes(data, context)?;

            let mut hasher = Sha256::new();
            hasher.update(&bytes);
            let result = hasher.finalize();

            bytes_to_js_array(&result, context)
        });
        context.register_global_builtin_callable(
            JsString::from("__acp_native_sha256"),
            1,
            sha256_fn
        ).map_err(|e| AcpError::plugin(format!("Failed to register sha256: {}", e)))?;

        // sha256Hex - returns hex string
        let sha256_hex_fn = NativeFunction::from_fn_ptr(|_, args, context| {
            let data = args.get_or_undefined(0);
            let bytes = js_value_to_bytes(data, context)?;

            let mut hasher = Sha256::new();
            hasher.update(&bytes);
            let result = hasher.finalize();

            Ok(JsValue::from(JsString::from(hex::encode(result))))
        });
        context.register_global_builtin_callable(
            JsString::from("__acp_native_sha256_hex"),
            1,
            sha256_hex_fn
        ).map_err(|e| AcpError::plugin(format!("Failed to register sha256Hex: {}", e)))?;

        // hmac - returns encoded string
        let hmac_fn = NativeFunction::from_fn_ptr(|_, args, context| {
            let key = args.get_or_undefined(0);
            let data = args.get_or_undefined(1);
            let encoding = args.get_or_undefined(2);

            let key_bytes = js_value_to_bytes(key, context)?;
            let data_bytes = js_value_to_bytes(data, context)?;
            let encoding_str = if let Some(s) = encoding.as_string() {
                s.to_std_string_escaped()
            } else {
                "hex".to_string()
            };

            let mut mac = Hmac::<Sha256>::new_from_slice(&key_bytes)
                .map_err(|e| JsNativeError::typ().with_message(format!("HMAC key error: {}", e)))?;
            mac.update(&data_bytes);
            let result = mac.finalize().into_bytes();

            match encoding_str.as_str() {
                "hex" => Ok(JsValue::from(JsString::from(hex::encode(result)))),
                "base64" => Ok(JsValue::from(JsString::from(base64::prelude::BASE64_STANDARD.encode(result)))),
                _ => bytes_to_js_array(&result, context),
            }
        });
        context.register_global_builtin_callable(
            JsString::from("__acp_native_hmac"),
            3,
            hmac_fn
        ).map_err(|e| AcpError::plugin(format!("Failed to register hmac: {}", e)))?;

        Ok(())
    }

    /// Register native util functions
    fn register_util_natives(context: &mut Context) -> Result<()> {
        use base64::prelude::*;

        // base64 encode
        let base64_encode_fn = NativeFunction::from_fn_ptr(|_, args, context| {
            let data = args.get_or_undefined(0);
            let bytes = js_value_to_bytes(data, context)?;
            Ok(JsValue::from(JsString::from(BASE64_STANDARD.encode(&bytes))))
        });
        context.register_global_builtin_callable(
            JsString::from("__acp_native_base64_encode"),
            1,
            base64_encode_fn
        ).map_err(|e| AcpError::plugin(format!("Failed to register base64 encode: {}", e)))?;

        // base64 decode
        let base64_decode_fn = NativeFunction::from_fn_ptr(|_, args, context| {
            let data = args.get_or_undefined(0);
            let s = if let Some(js_str) = data.as_string() {
                js_str.to_std_string_escaped()
            } else {
                return Err(JsNativeError::typ()
                    .with_message("Expected string for base64 decode")
                    .into());
            };

            let bytes = BASE64_STANDARD.decode(s.as_bytes())
                .map_err(|e| JsNativeError::typ().with_message(format!("Base64 decode error: {}", e)))?;

            bytes_to_js_array(&bytes, context)
        });
        context.register_global_builtin_callable(
            JsString::from("__acp_native_base64_decode"),
            1,
            base64_decode_fn
        ).map_err(|e| AcpError::plugin(format!("Failed to register base64 decode: {}", e)))?;

        // hex encode
        let hex_encode_fn = NativeFunction::from_fn_ptr(|_, args, context| {
            let data = args.get_or_undefined(0);
            let bytes = js_value_to_bytes(data, context)?;
            Ok(JsValue::from(JsString::from(hex::encode(&bytes))))
        });
        context.register_global_builtin_callable(
            JsString::from("__acp_native_hex_encode"),
            1,
            hex_encode_fn
        ).map_err(|e| AcpError::plugin(format!("Failed to register hex encode: {}", e)))?;

        // hex decode
        let hex_decode_fn = NativeFunction::from_fn_ptr(|_, args, context| {
            let data = args.get_or_undefined(0);
            let s = if let Some(js_str) = data.as_string() {
                js_str.to_std_string_escaped()
            } else {
                return Err(JsNativeError::typ()
                    .with_message("Expected string for hex decode")
                    .into());
            };

            let bytes = hex::decode(s)
                .map_err(|e| JsNativeError::typ().with_message(format!("Hex decode error: {}", e)))?;

            bytes_to_js_array(&bytes, context)
        });
        context.register_global_builtin_callable(
            JsString::from("__acp_native_hex_decode"),
            1,
            hex_decode_fn
        ).map_err(|e| AcpError::plugin(format!("Failed to register hex decode: {}", e)))?;

        // now - returns current timestamp in milliseconds
        let now_fn = NativeFunction::from_fn_ptr(|_, _, _| {
            let now = Utc::now().timestamp_millis();
            Ok(JsValue::from(now))
        });
        context.register_global_builtin_callable(
            JsString::from("__acp_native_now"),
            0,
            now_fn
        ).map_err(|e| AcpError::plugin(format!("Failed to register now: {}", e)))?;

        // isoDate - formats timestamp as ISO 8601 date string
        let iso_date_fn = NativeFunction::from_fn_ptr(|_, args, _| {
            let timestamp = args.get_or_undefined(0);
            let ts = timestamp.as_number()
                .ok_or_else(|| JsNativeError::typ().with_message("Expected number for timestamp"))?;

            let dt = chrono::DateTime::from_timestamp_millis(ts as i64)
                .ok_or_else(|| JsNativeError::typ().with_message("Invalid timestamp"))?;

            Ok(JsValue::from(JsString::from(dt.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string())))
        });
        context.register_global_builtin_callable(
            JsString::from("__acp_native_iso_date"),
            1,
            iso_date_fn
        ).map_err(|e| AcpError::plugin(format!("Failed to register isoDate: {}", e)))?;

        // amzDate - formats timestamp as AWS date string (YYYYMMDD'T'HHMMSS'Z')
        let amz_date_fn = NativeFunction::from_fn_ptr(|_, args, _| {
            let timestamp = args.get_or_undefined(0);
            let ts = timestamp.as_number()
                .ok_or_else(|| JsNativeError::typ().with_message("Expected number for timestamp"))?;

            let dt = chrono::DateTime::from_timestamp_millis(ts as i64)
                .ok_or_else(|| JsNativeError::typ().with_message("Invalid timestamp"))?;

            Ok(JsValue::from(JsString::from(dt.format("%Y%m%dT%H%M%SZ").to_string())))
        });
        context.register_global_builtin_callable(
            JsString::from("__acp_native_amz_date"),
            1,
            amz_date_fn
        ).map_err(|e| AcpError::plugin(format!("Failed to register amzDate: {}", e)))?;

        Ok(())
    }

    /// Set up TextEncoder and TextDecoder
    fn setup_text_encoding(context: &mut Context) -> Result<()> {
        // Register native encode/decode functions
        let encode_fn = NativeFunction::from_fn_ptr(|_, args, context| {
            let text = args.get_or_undefined(0);
            let s = if let Some(js_str) = text.as_string() {
                js_str.to_std_string_escaped()
            } else {
                return Err(JsNativeError::typ()
                    .with_message("Expected string")
                    .into());
            };

            bytes_to_js_array(s.as_bytes(), context)
        });
        context.register_global_builtin_callable(
            JsString::from("__acp_native_text_encode"),
            1,
            encode_fn
        ).map_err(|e| AcpError::plugin(format!("Failed to register text encode: {}", e)))?;

        let decode_fn = NativeFunction::from_fn_ptr(|_, args, context| {
            let data = args.get_or_undefined(0);
            let bytes = js_value_to_bytes(data, context)?;

            let s = String::from_utf8(bytes)
                .map_err(|e| JsNativeError::typ().with_message(format!("UTF-8 decode error: {}", e)))?;

            Ok(JsValue::from(JsString::from(s)))
        });
        context.register_global_builtin_callable(
            JsString::from("__acp_native_text_decode"),
            1,
            decode_fn
        ).map_err(|e| AcpError::plugin(format!("Failed to register text decode: {}", e)))?;

        // Create TextEncoder and TextDecoder classes
        let text_code = r#"
        function TextEncoder() {}
        TextEncoder.prototype.encode = function(str) {
            return __acp_native_text_encode(str);
        };

        function TextDecoder() {}
        TextDecoder.prototype.decode = function(bytes) {
            return __acp_native_text_decode(bytes);
        };
        "#;
        context.eval(Source::from_bytes(text_code))
            .map_err(|e| AcpError::plugin(format!("Failed to create TextEncoder/TextDecoder: {}", e)))?;

        Ok(())
    }

    /// Apply sandbox restrictions
    fn apply_sandbox(context: &mut Context) -> Result<()> {
        // Block dangerous globals by setting them to undefined or throwing functions
        let sandbox_code = r#"
        // Block network access
        if (typeof fetch !== 'undefined') {
            fetch = function() { throw new Error('fetch is not allowed in plugin sandbox'); };
        }
        if (typeof XMLHttpRequest !== 'undefined') {
            XMLHttpRequest = function() { throw new Error('XMLHttpRequest is not allowed in plugin sandbox'); };
        }

        // Block dynamic code evaluation
        if (typeof eval !== 'undefined') {
            eval = function() { throw new Error('eval is not allowed in plugin sandbox'); };
        }
        if (typeof Function !== 'undefined') {
            const OriginalFunction = Function;
            Function = function() { throw new Error('Function constructor is not allowed in plugin sandbox'); };
            Function.prototype = OriginalFunction.prototype;
        }

        // Block WebAssembly
        if (typeof WebAssembly !== 'undefined') {
            WebAssembly = undefined;
        }
        "#;

        context.eval(Source::from_bytes(sandbox_code))
            .map_err(|e| AcpError::plugin(format!("Failed to apply sandbox: {}", e)))?;

        Ok(())
    }
}

impl Default for PluginRuntime {
    fn default() -> Self {
        Self::new().expect("Failed to create default PluginRuntime")
    }
}

// Helper functions for converting between JS and Rust types

fn js_value_to_bytes(value: &JsValue, context: &mut Context) -> JsResult<Vec<u8>> {
    if let Some(s) = value.as_string() {
        // String -> UTF-8 bytes
        Ok(s.to_std_string_escaped().into_bytes())
    } else if let Some(obj) = value.as_object() {
        // Try to extract as array-like object
        let length_key = JsString::from("length");
        let length_value = obj.get(length_key, context)?;

        if let Some(length) = length_value.as_number() {
            let len = length as usize;
            let mut bytes = Vec::with_capacity(len);
            for i in 0..len {
                let val = obj.get(i, context)?;
                let byte = val.as_number()
                    .ok_or_else(|| JsNativeError::typ().with_message("Array element must be number"))? as u8;
                bytes.push(byte);
            }
            Ok(bytes)
        } else {
            Err(JsNativeError::typ()
                .with_message("Expected array-like object with length property")
                .into())
        }
    } else {
        Err(JsNativeError::typ()
            .with_message("Expected string or array-like object")
            .into())
    }
}

fn bytes_to_js_array(bytes: &[u8], context: &mut Context) -> JsResult<JsValue> {
    // Create a JavaScript array from bytes
    let array = context.eval(Source::from_bytes("[]"))?;
    let array_obj = array.as_object()
        .ok_or_else(|| JsNativeError::typ().with_message("Failed to create array"))?;

    for (i, &byte) in bytes.iter().enumerate() {
        array_obj.set(i, JsValue::from(byte as i32), false, context)?;
    }

    Ok(array)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_creation() {
        let runtime = PluginRuntime::new();
        assert!(runtime.is_ok());
    }

    #[test]
    fn test_basic_javascript_execution() {
        let mut runtime = PluginRuntime::new().unwrap();
        let result = runtime.execute("1 + 1").unwrap();
        assert_eq!(result.as_number().unwrap(), 2.0);
    }

    #[test]
    fn test_acp_crypto_sha256_hex_exists() {
        let mut runtime = PluginRuntime::new().unwrap();
        let result = runtime.execute("typeof ACP.crypto.sha256Hex").unwrap();
        assert_eq!(result.as_string().unwrap().to_std_string_escaped(), "function");
    }

    #[test]
    fn test_acp_crypto_sha256_hex() {
        let mut runtime = PluginRuntime::new().unwrap();
        let result = runtime.execute("ACP.crypto.sha256Hex('hello')").unwrap();
        let hash = result.as_string().unwrap().to_std_string_escaped();

        // Expected SHA-256 of "hello"
        assert_eq!(hash, "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824");
    }

    #[test]
    fn test_acp_crypto_hmac() {
        let mut runtime = PluginRuntime::new().unwrap();
        let result = runtime.execute("ACP.crypto.hmac('key', 'message', 'hex')").unwrap();
        let hmac = result.as_string().unwrap().to_std_string_escaped();

        // Expected HMAC-SHA256 of "message" with key "key"
        assert_eq!(hmac, "6e9ef29b75fffc5b7abae527d58fdadb2fe42e7219011976917343065f58ed4a");
    }

    #[test]
    fn test_acp_util_base64() {
        let mut runtime = PluginRuntime::new().unwrap();
        let result = runtime.execute("ACP.util.base64('hello')").unwrap();
        let encoded = result.as_string().unwrap().to_std_string_escaped();
        assert_eq!(encoded, "aGVsbG8=");
    }

    #[test]
    fn test_acp_util_hex() {
        let mut runtime = PluginRuntime::new().unwrap();
        let result = runtime.execute("ACP.util.hex('hello')").unwrap();
        let encoded = result.as_string().unwrap().to_std_string_escaped();
        assert_eq!(encoded, "68656c6c6f");
    }

    #[test]
    fn test_acp_util_now() {
        let mut runtime = PluginRuntime::new().unwrap();
        let result = runtime.execute("ACP.util.now()").unwrap();
        let now = result.as_number().unwrap();

        // Should be a reasonable timestamp (after 2020)
        assert!(now > 1577836800000.0);
    }

    #[test]
    fn test_acp_util_iso_date() {
        let mut runtime = PluginRuntime::new().unwrap();
        // Use a fixed timestamp: 2024-01-01 00:00:00 UTC = 1704067200000 ms
        let result = runtime.execute("ACP.util.isoDate(1704067200000)").unwrap();
        let date = result.as_string().unwrap().to_std_string_escaped();
        assert!(date.starts_with("2024-01-01T00:00:00"));
    }

    #[test]
    fn test_acp_util_amz_date() {
        let mut runtime = PluginRuntime::new().unwrap();
        // Use a fixed timestamp: 2024-01-01 00:00:00 UTC = 1704067200000 ms
        let result = runtime.execute("ACP.util.amzDate(1704067200000)").unwrap();
        let date = result.as_string().unwrap().to_std_string_escaped();
        assert_eq!(date, "20240101T000000Z");
    }

    #[test]
    fn test_text_encoder() {
        let mut runtime = PluginRuntime::new().unwrap();
        let result = runtime.execute("new TextEncoder().encode('hello')").unwrap();
        // Result should be an array
        assert!(result.is_object());
    }

    #[test]
    fn test_text_decoder() {
        let mut runtime = PluginRuntime::new().unwrap();
        let result = runtime.execute("new TextDecoder().decode([104, 101, 108, 108, 111])").unwrap();
        let decoded = result.as_string().unwrap().to_std_string_escaped();
        assert_eq!(decoded, "hello");
    }

    #[test]
    fn test_sandbox_blocks_fetch() {
        let mut runtime = PluginRuntime::new().unwrap();
        let result = runtime.execute("fetch('https://example.com')");
        assert!(result.is_err());
    }

    #[test]
    fn test_sandbox_blocks_eval() {
        let mut runtime = PluginRuntime::new().unwrap();
        let result = runtime.execute("eval('1 + 1')");
        assert!(result.is_err());
    }

    #[test]
    fn test_sandbox_blocks_function_constructor() {
        let mut runtime = PluginRuntime::new().unwrap();
        let result = runtime.execute("new Function('return 1')()");
        assert!(result.is_err());
    }
}
