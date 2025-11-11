use std::{collections::HashMap, sync::RwLock};

use evalexpr::{error::EvalexprResultValue, *};

use crate::app::calc::Grid;

pub struct CallbackContext<'a, T: EvalexprNumericTypes = DefaultNumericTypes> {
    variables: &'a Grid,
    eval_depth: RwLock<usize>,
    functions: HashMap<String, Function<T>>,

    /// True if builtin functions are disabled.
    without_builtin_functions: bool,
}

impl<'a, NumericTypes: EvalexprNumericTypes> CallbackContext<'a, NumericTypes> {
    pub fn new(grid: &'a Grid) -> Self {
        Self {
            eval_depth: RwLock::new(0),
            variables: grid,
            functions: Default::default(),
            without_builtin_functions: false,
        }
    }

    pub fn clear_variables(&mut self) {
        ()
    }

    pub fn clear_functions(&mut self) {
        self.functions.clear()
    }

    pub fn clear(&mut self) {
        self.clear_variables();
        self.clear_functions();
    }
}

impl<'a> Context for CallbackContext<'a, DefaultNumericTypes> {
    type NumericTypes = DefaultNumericTypes;

    fn get_value(&self, identifier: &str) -> Option<Value<Self::NumericTypes>> {
        if let Some(v) = self.variables.get_cell(identifier) {

            match v {
                super::calc::CellType::Number(n) => return Some(Value::Float(n.to_owned())),
                super::calc::CellType::String(s) => unimplemented!("{s}"),
                super::calc::CellType::Equation(eq) => {
                    if let Ok(mut depth) = self.eval_depth.write() {
                        *depth += 1;
                    }
                    if let Ok(depth) = self.eval_depth.read() {
                        if *depth > 10 {
                            return None
                        }                        
                    } else {
                        // It would be unsafe to continue to process without knowing how
                        // deep we've gone.
                        return None
                    }
                    match eval_with_context(&eq[1..], self) {
                        Ok(e) => return Some(e),
                        Err(e) => {
                            match e {
                                EvalexprError::VariableIdentifierNotFound(_) => {
                                    // If the variable isn't found, that's ~~probably~~ because
                                    // of recursive reference, considering all references
                                    // are grabbed straight from the table.
                                    return None
                                },

                               e => panic!("> Error {e}\n> Equation: '{eq}'"),
                            }
                        },
                    }
                },
            }
        }
        return None;
    }

    fn call_function(
        &self,
        identifier: &str,
        argument: &Value<Self::NumericTypes>,
    ) -> EvalexprResultValue<Self::NumericTypes> {
        todo!()
    }

    fn are_builtin_functions_disabled(&self) -> bool {
        self.without_builtin_functions
    }

    fn set_builtin_functions_disabled(
        &mut self,
        disabled: bool,
    ) -> EvalexprResult<(), Self::NumericTypes> {
        self.without_builtin_functions = disabled;
        Ok(())
    }
}
