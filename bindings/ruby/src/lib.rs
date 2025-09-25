use magnus::{function, prelude::*, Error, Ruby, RString};
use helix::{parse_and_validate, load_file, pretty_print, parse};

/// Parse and validate HLX source code
fn parse_hlx(_ruby: &Ruby, source: RString) -> Result<RString, Error> {
    let source_str = unsafe { source.as_str()? };

    match parse_and_validate(source_str) {
        Ok(_) => Ok(RString::new("HLX configuration parsed and validated successfully")),
        Err(e) => Err(Error::new(magnus::exception::runtime_error(), e)),
    }
}

/// Load and parse an HLX file
fn load_hlx_file(_ruby: &Ruby, path: RString) -> Result<RString, Error> {
    let path_str = unsafe { path.as_str()? };

    match load_file(path_str) {
        Ok(_) => Ok(RString::new("HLX file loaded and parsed successfully")),
        Err(e) => Err(Error::new(magnus::exception::runtime_error(), e)),
    }
}

/// Pretty print an HLX AST (for demonstration)
fn pretty_print_hlx(_ruby: &Ruby, source: RString) -> Result<RString, Error> {
    let source_str = unsafe { source.as_str()? };

    match parse(source_str) {
        Ok(ast) => {
            let pretty = pretty_print(&ast);
            Ok(RString::new(&pretty))
        }
        Err(e) => Err(Error::new(magnus::exception::runtime_error(), format!("Parse error: {:?}", e))),
    }
}

/// The main "do_thing" function as requested
fn do_thing(_ruby: &Ruby, arg: RString) -> Result<RString, Error> {
    let arg_str = unsafe { arg.as_str()? };

    match parse_and_validate(arg_str) {
        Ok(_) => Ok(RString::new("Successfully processed HLX configuration")),
        Err(e) => Err(Error::new(magnus::exception::runtime_error(), e)),
    }
}

#[magnus::init]
fn init(ruby: &Ruby) -> Result<(), Error> {
    let module = ruby.define_module("Helix")?;
    module.define_singleton_method("parse", function!(parse_hlx, 1))?;
    module.define_singleton_method("load_file", function!(load_hlx_file, 1))?;
    module.define_singleton_method("pretty_print", function!(pretty_print_hlx, 1))?;
    module.define_singleton_method("do_thing", function!(do_thing, 1))?;
    Ok(())
}