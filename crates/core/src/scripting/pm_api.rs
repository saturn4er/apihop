/// Generate JS code that sets up the `pm` global for pre-request scripts.
pub fn pre_request_pm_js(request_json: &str, variables_json: &str) -> String {
    format!(
        r#"
var __pm_variables = {{}};
var __pm_env_updates = {{}};
var __pm_error = null;
var __pm_request = JSON.parse({request_json});
var __pm_vars_data = JSON.parse({vars_json});

var pm = {{
    variables: {{
        get: function(key) {{
            if (__pm_variables.hasOwnProperty(key)) return __pm_variables[key];
            if (__pm_vars_data.hasOwnProperty(key)) return __pm_vars_data[key];
            return undefined;
        }},
        set: function(key, val) {{
            __pm_variables[key] = String(val);
            __pm_vars_data[key] = String(val);
        }}
    }},
    environment: {{
        set: function(key, val) {{
            __pm_env_updates[key] = String(val);
        }}
    }},
    request: {{
        get url() {{ return __pm_request.url; }},
        set url(v) {{ __pm_request.url = v; }},
        get body() {{ return __pm_request.body || null; }},
        set body(v) {{ __pm_request.body = v; }},
        get headers() {{
            var h = __pm_request.headers || {{}};
            return {{
                add: function(key, val) {{ h[key] = String(val); __pm_request.headers = h; }},
                get: function(key) {{
                    for (var k in h) {{ if (k.toLowerCase() === key.toLowerCase()) return h[k]; }}
                    return undefined;
                }},
                remove: function(key) {{
                    for (var k in h) {{ if (k.toLowerCase() === key.toLowerCase()) delete h[k]; }}
                    __pm_request.headers = h;
                }},
                toObject: function() {{ return h; }}
            }};
        }},
        set headers(v) {{ __pm_request.headers = v; }}
    }}
}};
"#,
        request_json = escape_js_string(request_json),
        vars_json = escape_js_string(variables_json),
    )
}

/// JS code appended after user script to collect pre-request results as JSON.
pub const PRE_REQUEST_COLLECT_JS: &str = r#"
(function() {
    var result = {
        console: __console_entries,
        variables: __pm_variables,
        environment: __pm_env_updates,
        request: {
            url: __pm_request.url,
            headers: __pm_request.headers || {},
            body: __pm_request.body || null
        },
        error: __pm_error
    };
    return JSON.stringify(result);
})()
"#;

/// Generate JS code that sets up the `pm` global for test scripts.
pub fn test_pm_js(response_json: &str, variables_json: &str) -> String {
    format!(
        r#"
var __pm_variables = {{}};
var __pm_env_updates = {{}};
var __pm_error = null;
var __pm_test_results = [];
var __pm_response = JSON.parse({response_json});
var __pm_vars_data = JSON.parse({vars_json});

function __make_expect(val) {{
    var obj = {{
        to: {{
            get equal() {{
                return function(expected) {{
                    if (val !== expected) {{
                        throw new Error("Expected " + JSON.stringify(expected) + " but got " + JSON.stringify(val));
                    }}
                }};
            }},
            get include() {{
                return function(expected) {{
                    if (typeof val === 'string') {{
                        if (val.indexOf(expected) === -1) {{
                            throw new Error("Expected string to include " + JSON.stringify(expected));
                        }}
                    }} else if (Array.isArray(val)) {{
                        if (val.indexOf(expected) === -1) {{
                            throw new Error("Expected array to include " + JSON.stringify(expected));
                        }}
                    }} else if (typeof val === 'object' && val !== null) {{
                        if (!val.hasOwnProperty(expected)) {{
                            throw new Error("Expected object to have key " + JSON.stringify(expected));
                        }}
                    }} else {{
                        throw new Error("Cannot check inclusion on " + typeof val);
                    }}
                }};
            }},
            get have() {{
                return {{
                    status: function(code) {{
                        if (val !== code) {{
                            throw new Error("Expected status " + code + " but got " + val);
                        }}
                    }}
                }};
            }},
            get be() {{
                return {{
                    get true() {{ if (val !== true) throw new Error("Expected true but got " + JSON.stringify(val)); }},
                    get false() {{ if (val !== false) throw new Error("Expected false but got " + JSON.stringify(val)); }},
                    get null() {{ if (val !== null) throw new Error("Expected null but got " + JSON.stringify(val)); }},
                    get undefined() {{ if (val !== undefined) throw new Error("Expected undefined but got " + JSON.stringify(val)); }},
                    above: function(n) {{ if (!(val > n)) throw new Error("Expected " + val + " to be above " + n); }},
                    below: function(n) {{ if (!(val < n)) throw new Error("Expected " + val + " to be below " + n); }}
                }};
            }}
        }}
    }};
    return obj;
}}

var pm = {{
    variables: {{
        get: function(key) {{
            if (__pm_variables.hasOwnProperty(key)) return __pm_variables[key];
            if (__pm_vars_data.hasOwnProperty(key)) return __pm_vars_data[key];
            return undefined;
        }},
        set: function(key, val) {{
            __pm_variables[key] = String(val);
            __pm_vars_data[key] = String(val);
        }}
    }},
    environment: {{
        set: function(key, val) {{
            __pm_env_updates[key] = String(val);
        }}
    }},
    response: {{
        get status() {{ return __pm_response.status; }},
        get body() {{ return __pm_response.body || ""; }},
        get headers() {{
            var h = __pm_response.headers || {{}};
            return {{
                get: function(key) {{
                    for (var k in h) {{ if (k.toLowerCase() === key.toLowerCase()) return h[k]; }}
                    return undefined;
                }},
                toObject: function() {{ return h; }}
            }};
        }},
        json: function() {{
            try {{
                return JSON.parse(__pm_response.body || "null");
            }} catch(e) {{
                throw new Error("Response body is not valid JSON: " + e.message);
            }}
        }}
    }},
    test: function(name, fn) {{
        try {{
            fn();
            __pm_test_results.push({{ name: name, passed: true, error: null }});
        }} catch(e) {{
            __pm_test_results.push({{ name: name, passed: false, error: e.message || String(e) }});
        }}
    }},
    expect: function(val) {{
        return __make_expect(val);
    }}
}};
"#,
        response_json = escape_js_string(response_json),
        vars_json = escape_js_string(variables_json),
    )
}

/// JS code appended after user script to collect test results as JSON.
pub const TEST_COLLECT_JS: &str = r#"
(function() {
    var result = {
        console: __console_entries,
        variables: __pm_variables,
        environment: __pm_env_updates,
        tests: __pm_test_results,
        error: __pm_error
    };
    return JSON.stringify(result);
})()
"#;

/// Escape a string for safe embedding in JS as a string literal.
fn escape_js_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\0' => out.push_str("\\0"),
            c if c < ' ' => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
