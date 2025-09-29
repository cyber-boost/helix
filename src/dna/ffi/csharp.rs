use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::ptr;
#[cfg(feature = "csharp")]
use dotnet_bindgen;
use crate::Parser;
use crate::SemanticAnalyzer;
use crate::HelixConfig as RustHelixConfig;
use serde_json;
#[repr(C)]
pub struct HelixConfigFFI {
    ptr: *mut RustHelixConfig,
}
#[cfg(feature = "csharp")]
#[dotnet_bindgen]
#[no_mangle]
pub unsafe extern "C" fn helix_parse(
    input: *const c_char,
    error_code: *mut c_int,
    error_message: *mut *mut c_char,
) -> *mut HelixConfigFFI {
    if input.is_null() {
        *error_code = -1;
        *error_message = CString::new("Input is null").unwrap().into_raw();
        return ptr::null_mut();
    }
    let input_str = match CStr::from_ptr(input).to_str() {
        Ok(s) => s,
        Err(_) => {
            *error_code = -2;
            *error_message = CString::new("Invalid UTF-8 input").unwrap().into_raw();
            return ptr::null_mut();
        }
    };
    let mut parser = match Parser::new(input_str) {
        Ok(p) => p,
        Err(e) => {
            *error_code = -3;
            *error_message = CString::new(format!("Parser creation failed: {}", e))
                .unwrap()
                .into_raw();
            return ptr::null_mut();
        }
    };
    let ast = match parser.parse() {
        Ok(a) => a,
        Err(e) => {
            *error_code = -4;
            *error_message = CString::new(format!("Parse error: {}", e))
                .unwrap()
                .into_raw();
            return ptr::null_mut();
        }
    };
    let mut analyzer = SemanticAnalyzer::new();
    let config = match analyzer.analyze(&ast) {
        Ok(c) => c,
        Err(e) => {
            *error_code = -5;
            *error_message = CString::new(format!("Semantic analysis failed: {}", e))
                .unwrap()
                .into_raw();
            return ptr::null_mut();
        }
    };
    *error_code = 0;
    *error_message = ptr::null_mut();
    let ffi_config = Box::new(HelixConfigFFI {
        ptr: Box::into_raw(Box::new(config)),
    });
    Box::into_raw(ffi_config)
}
#[cfg(feature = "csharp")]
#[dotnet_bindgen]
#[no_mangle]
pub unsafe extern "C" fn helix_free_config(config: *mut HelixConfigFFI) {
    if config.is_null() {
        return;
    }
    let ffi_config = Box::from_raw(config);
    if !ffi_config.ptr.is_null() {
        let _ = Box::from_raw(ffi_config.ptr);
    }
}
#[cfg(feature = "csharp")]
#[dotnet_bindgen]
#[no_mangle]
pub unsafe extern "C" fn helix_free_string(s: *mut c_char) {
    if !s.is_null() {
        let _ = CString::from_raw(s);
    }
}
#[cfg(feature = "csharp")]
#[dotnet_bindgen]
#[no_mangle]
pub unsafe extern "C" fn helix_config_get_agents(
    config: *const HelixConfigFFI,
) -> *mut c_char {
    if config.is_null() {
        return ptr::null_mut();
    }
    let ffi_config = &*config;
    if ffi_config.ptr.is_null() {
        return ptr::null_mut();
    }
    let rust_config = &*ffi_config.ptr;
    let agents_json = serde_json::to_string(&rust_config.agents)
        .unwrap_or_else(|_| "{}".to_string());
    CString::new(agents_json).unwrap().into_raw()
}
#[cfg(feature = "csharp")]
#[dotnet_bindgen]
#[no_mangle]
pub unsafe extern "C" fn helix_config_get_workflows(
    config: *const HelixConfigFFI,
) -> *mut c_char {
    if config.is_null() {
        return ptr::null_mut();
    }
    let ffi_config = &*config;
    if ffi_config.ptr.is_null() {
        return ptr::null_mut();
    }
    let rust_config = &*ffi_config.ptr;
    let workflows_json = serde_json::to_string(&rust_config.workflows)
        .unwrap_or_else(|_| "{}".to_string());
    CString::new(workflows_json).unwrap().into_raw()
}
#[cfg(feature = "csharp")]
#[dotnet_bindgen]
#[no_mangle]
pub unsafe extern "C" fn helix_config_get_memories(
    config: *const HelixConfigFFI,
) -> *mut c_char {
    if config.is_null() {
        return ptr::null_mut();
    }
    let ffi_config = &*config;
    if ffi_config.ptr.is_null() {
        return ptr::null_mut();
    }
    let rust_config = &*ffi_config.ptr;
    let memories_json = serde_json::to_string(&rust_config.memories)
        .unwrap_or_else(|_| "{}".to_string());
    CString::new(memories_json).unwrap().into_raw()
}
#[cfg(feature = "csharp")]
#[dotnet_bindgen]
#[no_mangle]
pub unsafe extern "C" fn helix_config_get_contexts(
    config: *const HelixConfigFFI,
) -> *mut c_char {
    if config.is_null() {
        return ptr::null_mut();
    }
    let ffi_config = &*config;
    if ffi_config.ptr.is_null() {
        return ptr::null_mut();
    }
    let rust_config = &*ffi_config.ptr;
    let contexts_json = serde_json::to_string(&rust_config.contexts)
        .unwrap_or_else(|_| "{}".to_string());
    CString::new(contexts_json).unwrap().into_raw()
}
#[cfg(feature = "csharp")]
#[dotnet_bindgen]
#[no_mangle]
pub unsafe extern "C" fn helix_config_get_crews(
    config: *const HelixConfigFFI,
) -> *mut c_char {
    if config.is_null() {
        return ptr::null_mut();
    }
    let ffi_config = &*config;
    if ffi_config.ptr.is_null() {
        return ptr::null_mut();
    }
    let rust_config = &*ffi_config.ptr;
    let crews_json = serde_json::to_string(&rust_config.crews)
        .unwrap_or_else(|_| "{}".to_string());
    CString::new(crews_json).unwrap().into_raw()
}
#[cfg(feature = "csharp")]
#[dotnet_bindgen]
#[no_mangle]
pub unsafe extern "C" fn helix_config_get_pipelines(
    config: *const HelixConfigFFI,
) -> *mut c_char {
    if config.is_null() {
        return ptr::null_mut();
    }
    let ffi_config = &*config;
    if ffi_config.ptr.is_null() {
        return ptr::null_mut();
    }
    let rust_config = &*ffi_config.ptr;
    let pipelines_json = serde_json::to_string(&rust_config.pipelines)
        .unwrap_or_else(|_| "{}".to_string());
    CString::new(pipelines_json).unwrap().into_raw()
}
#[cfg(feature = "csharp")]
#[dotnet_bindgen]
#[no_mangle]
pub unsafe extern "C" fn helix_config_get_agent(
    config: *const HelixConfigFFI,
    name: *const c_char,
) -> *mut c_char {
    if config.is_null() || name.is_null() {
        return ptr::null_mut();
    }
    let ffi_config = &*config;
    if ffi_config.ptr.is_null() {
        return ptr::null_mut();
    }
    let name_str = match CStr::from_ptr(name).to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };
    let rust_config = &*ffi_config.ptr;
    if let Some(agent) = rust_config.agents.get(name_str) {
        let agent_json = serde_json::to_string(agent)
            .unwrap_or_else(|_| "{}".to_string());
        CString::new(agent_json).unwrap().into_raw()
    } else {
        ptr::null_mut()
    }
}
#[cfg(feature = "csharp")]
#[dotnet_bindgen]
#[no_mangle]
pub unsafe extern "C" fn helix_config_get_workflow(
    config: *const HelixConfigFFI,
    name: *const c_char,
) -> *mut c_char {
    if config.is_null() || name.is_null() {
        return ptr::null_mut();
    }
    let ffi_config = &*config;
    if ffi_config.ptr.is_null() {
        return ptr::null_mut();
    }
    let name_str = match CStr::from_ptr(name).to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };
    let rust_config = &*ffi_config.ptr;
    if let Some(workflow) = rust_config.workflows.get(name_str) {
        let workflow_json = serde_json::to_string(workflow)
            .unwrap_or_else(|_| "{}".to_string());
        CString::new(workflow_json).unwrap().into_raw()
    } else {
        ptr::null_mut()
    }
}