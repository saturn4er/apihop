use rquickjs::{Context, Runtime};

const MAX_MEMORY_BYTES: usize = 16 * 1024 * 1024; // 16 MB

/// Execute JavaScript code in a sandboxed QuickJS runtime.
/// Returns the JSON string result or an error message.
pub fn execute_js(code: &str) -> Result<String, String> {
    let runtime = Runtime::new().map_err(|e| format!("Failed to create JS runtime: {e}"))?;
    runtime.set_memory_limit(MAX_MEMORY_BYTES);
    runtime.set_max_stack_size(512 * 1024); // 512 KB stack

    let context = Context::full(&runtime).map_err(|e| format!("Failed to create JS context: {e}"))?;

    context.with(|ctx| {
        let result: rquickjs::Result<rquickjs::Value> = ctx.eval(code);
        match result {
            Ok(val) => {
                // The script returns a JSON string as the last expression
                if let Some(s) = val.as_string() {
                    s.to_string()
                        .map_err(|e| format!("Failed to read result string: {e}"))
                } else {
                    // Try to convert to string
                    val.get::<String>()
                        .map_err(|e| format!("Script did not return a string: {e}"))
                }
            }
            Err(e) => Err(format!("Script error: {e}")),
        }
    })
}
