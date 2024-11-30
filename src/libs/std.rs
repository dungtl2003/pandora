use std::collections::HashMap;

use crate::interpreter::eval::Value;

use super::Library;
use ::std::io::{self, Write};

pub struct StdLib {
    pub functions: HashMap<String, Box<dyn Fn(Vec<(Value, bool)>) -> Result<Value, String>>>,
}

impl Library for StdLib {
    fn get_function(
        &self,
        name: &str,
    ) -> Option<&Box<dyn Fn(Vec<(Value, bool)>) -> Result<Value, String>>> {
        self.functions.get(name)
    }
}

impl StdLib {
    pub fn new() -> Self {
        let mut lib = StdLib {
            functions: HashMap::new(),
        };

        lib.register_functions();
        lib
    }

    fn register_functions(&mut self) {
        self.register_input_function();
        self.register_print_function();
        self.register_println_function();

        self.register_lower_function();
        self.register_upper_function();

        self.register_str_len_function();
        self.register_array_len_function();

        self.register_delay_function();
    }

    fn register_delay_function(&mut self) {
        // delay() function
        self.functions.insert(
            "delay".to_string(),
            Box::new(|args| {
                if args.len() != 1 {
                    return Err("delay() takes exactly 1 argument".to_string());
                }
                match &args[0].0 {
                    Value::Int(ms) => {
                        std::thread::sleep(std::time::Duration::from_millis(*ms as u64));
                    }
                    _ => {
                        return Err("delay() takes an integer".to_string());
                    }
                }
                Ok(Value::Unit)
            }),
        );
    }

    fn register_str_len_function(&mut self) {
        // strlen() function for strings
        self.functions.insert(
            "strlen".to_string(),
            Box::new(|args| {
                if args.len() != 1 {
                    return Err("strlen() takes exactly 1 argument".to_string());
                }
                match &args[0].0 {
                    Value::Str(s) => Ok(Value::Int(s.len() as i64)),
                    _ => Err("strlen() takes a string".to_string()),
                }
            }),
        );
    }

    fn register_array_len_function(&mut self) {
        // arrlen() function for arrays
        self.functions.insert(
            "arrlen".to_string(),
            Box::new(|args| {
                if args.len() != 1 {
                    return Err("arrlen() takes exactly 1 argument".to_string());
                }
                match &args[0].0 {
                    Value::Array(arr) => Ok(Value::Int(arr.len() as i64)),
                    _ => Err("arrlen() takes an array".to_string()),
                }
            }),
        );
    }

    fn register_lower_function(&mut self) {
        // lower() function
        self.functions.insert(
            "lower".to_string(),
            Box::new(|args| {
                if args.len() != 1 {
                    return Err("lower() takes exactly 1 argument".to_string());
                }
                match &args[0].0 {
                    Value::Str(s) => Ok(Value::Str(s.to_lowercase())),
                    _ => Err("lower() takes a string".to_string()),
                }
            }),
        );
    }

    fn register_upper_function(&mut self) {
        // upper() function
        self.functions.insert(
            "upper".to_string(),
            Box::new(|args| {
                if args.len() != 1 {
                    return Err("upper() takes exactly 1 argument".to_string());
                }
                match &args[0].0 {
                    Value::Str(s) => Ok(Value::Str(s.to_uppercase())),
                    _ => Err("upper() takes a string".to_string()),
                }
            }),
        );
    }

    fn register_input_function(&mut self) {
        // input() function
        self.functions.insert(
            "input".to_string(),
            Box::new(|args| {
                if !args.is_empty() {
                    return Err("input() takes no arguments".to_string());
                }
                let mut input = String::new();
                std::io::stdin()
                    .read_line(&mut input)
                    .expect("Failed to read line");
                Ok(Value::Str(input.trim().to_string()))
            }),
        );
    }

    fn register_println_function(&mut self) {
        // println() function
        self.functions.insert(
            "println".to_string(),
            Box::new(|args| {
                if args.len() != 1 {
                    return Err("println() takes exactly 1 argument".to_string());
                }
                match &args[0].0 {
                    Value::Str(s) => {
                        println!("{}", s);
                    }
                    _ => {
                        return Err("println() takes a string".to_string());
                    }
                }
                Ok(Value::Unit)
            }),
        );
    }

    fn register_print_function(&mut self) {
        // print() function
        self.functions.insert(
            "print".to_string(),
            Box::new(|args| {
                if args.len() != 1 {
                    return Err("print() takes exactly 1 argument".to_string());
                }
                match &args[0].0 {
                    Value::Str(s) => {
                        print!("{}", s);
                    }
                    _ => {
                        return Err("print() takes a string".to_string());
                    }
                }
                io::stdout().flush().map_err(|e| e.to_string())?;
                Ok(Value::Unit)
            }),
        );
    }
}
