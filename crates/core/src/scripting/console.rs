/// JavaScript preamble that replaces console.log/warn/error with capturing versions.
pub const CONSOLE_SETUP_JS: &str = r#"
var __console_entries = [];
var console = {
    log: function() {
        var parts = [];
        for (var i = 0; i < arguments.length; i++) {
            var arg = arguments[i];
            if (typeof arg === 'object' && arg !== null) {
                try { parts.push(JSON.stringify(arg)); } catch(e) { parts.push(String(arg)); }
            } else {
                parts.push(String(arg));
            }
        }
        __console_entries.push({ level: "log", message: parts.join(" ") });
    },
    warn: function() {
        var parts = [];
        for (var i = 0; i < arguments.length; i++) {
            var arg = arguments[i];
            if (typeof arg === 'object' && arg !== null) {
                try { parts.push(JSON.stringify(arg)); } catch(e) { parts.push(String(arg)); }
            } else {
                parts.push(String(arg));
            }
        }
        __console_entries.push({ level: "warn", message: parts.join(" ") });
    },
    error: function() {
        var parts = [];
        for (var i = 0; i < arguments.length; i++) {
            var arg = arguments[i];
            if (typeof arg === 'object' && arg !== null) {
                try { parts.push(JSON.stringify(arg)); } catch(e) { parts.push(String(arg)); }
            } else {
                parts.push(String(arg));
            }
        }
        __console_entries.push({ level: "error", message: parts.join(" ") });
    },
    info: function() {
        var parts = [];
        for (var i = 0; i < arguments.length; i++) {
            var arg = arguments[i];
            if (typeof arg === 'object' && arg !== null) {
                try { parts.push(JSON.stringify(arg)); } catch(e) { parts.push(String(arg)); }
            } else {
                parts.push(String(arg));
            }
        }
        __console_entries.push({ level: "info", message: parts.join(" ") });
    }
};
"#;
