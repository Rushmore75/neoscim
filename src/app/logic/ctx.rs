use std::{collections::HashMap, process::id, sync::RwLock};

use evalexpr::{error::EvalexprResultValue, *};

use crate::app::logic::calc::Grid;

pub struct CallbackContext<'a> {
    variables: &'a Grid,
    eval_depth: RwLock<usize>,
    functions: HashMap<String, Function<DefaultNumericTypes>>,

    /// True if builtin functions are disabled.
    without_builtin_functions: bool,
}

impl<'a> CallbackContext<'a> {
    fn get_functions() -> HashMap<String, Function<DefaultNumericTypes>> {
        let mut functions = HashMap::new();

        functions.insert(
            "avg".to_string(),
            Function::new(|arg| {
                if arg.is_tuple() {
                    let args = arg.as_tuple()?;
                    let mut total: f64 = 0.;
                    let mut count = 0f64;
                    for i in args {
                        total += i.as_number()?;
                        count += 1.;
                    }
                    let res = total / count;
                    Ok(Value::Float(res))
                } else if arg.is_float() {
                    Ok(Value::Float(arg.as_float()?))
                } else {
                    Err(EvalexprError::TypeError {
                        expected: vec![ValueType::Float],
                        actual: arg.clone(),
                    })
                }
            }),
        );

        functions.insert(
            "sum".to_string(),
            Function::new(|arg| {
                if arg.is_tuple() {
                    let args = arg.as_tuple()?;
                    let mut total: f64 = 0.;
                    for i in args {
                        total += i.as_number()?;
                    }
                    let res = total;
                    Ok(Value::Float(res))
                } else if arg.is_float() {
                    Ok(Value::Float(arg.as_float()?))
                } else {
                    Err(EvalexprError::TypeError {
                        expected: vec![ValueType::Float],
                        actual: arg.clone(),
                    })
                }
            }),
        );

        functions
    }

    pub fn new(grid: &'a Grid) -> Self {
        Self {
            eval_depth: RwLock::new(0),
            variables: grid,
            functions: Self::get_functions(),
            without_builtin_functions: false,
        }
    }

    #[allow(dead_code)]
    pub fn clear_variables(&mut self) {
        ()
    }

    #[allow(dead_code)]
    pub fn clear_functions(&mut self) {
        ()
    }

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.clear_variables();
        self.clear_functions();
    }
}

impl<'a> Context for CallbackContext<'a> {
    type NumericTypes = DefaultNumericTypes;

    fn get_value(&self, identifier: &str) -> Option<Value<Self::NumericTypes>> {
        if let Some(v) = self.variables.get_cell(identifier) {
            match v {
                super::calc::CellType::Number(n) => return Some(Value::Float(n.to_owned())),
                super::calc::CellType::String(_) => return None,
                super::calc::CellType::Equation(eq) => {
                    if let Ok(mut depth) = self.eval_depth.write() {
                        *depth += 1;
                    }
                    if let Ok(depth) = self.eval_depth.read() {
                        if *depth > 10 {
                            return None;
                        }
                    } else {
                        // It would be unsafe to continue to process without knowing how
                        // deep we've gone.
                        return None;
                    }
                    match eval_with_context(&eq[1..], self) {
                        Ok(e) => return Some(e),
                        Err(e) => {
                            match e {
                                EvalexprError::VariableIdentifierNotFound(_) => {
                                    // If the variable isn't found, that's ~~probably~~ because
                                    // of recursive reference, considering all references
                                    // are grabbed straight from the table.
                                    return None;
                                }

                                e => panic!("> Error {e}\n> Equation: '{eq}'"),
                            }
                        }
                    }
                }
            }
        }
        return None;
    }

    fn call_function(
        &self,
        identifier: &str,
        argument: &Value<Self::NumericTypes>,
    ) -> EvalexprResultValue<Self::NumericTypes> {
        if let Some(function) = self.functions.get(identifier) {
            function.call(argument)
        } else {
            Err(EvalexprError::FunctionIdentifierNotFound(identifier.to_string()))
        }
    }

    fn are_builtin_functions_disabled(&self) -> bool {
        self.without_builtin_functions
    }

    fn set_builtin_functions_disabled(&mut self, disabled: bool) -> EvalexprResult<(), Self::NumericTypes> {
        self.without_builtin_functions = disabled;
        Ok(())
    }
}
