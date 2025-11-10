use std::{collections::HashMap, rc::Rc};

use evalexpr::{error::EvalexprResultValue, *};

use crate::Grid;

pub struct CallbackContext<'a, T: EvalexprNumericTypes = DefaultNumericTypes> {
    variables: &'a Grid,
    functions: HashMap<String, Function<T>>,

    /// True if builtin functions are disabled.
    without_builtin_functions: bool,
}

impl<'a, NumericTypes: EvalexprNumericTypes> CallbackContext<'a, NumericTypes> {
    pub fn new(grid: &'a Grid) -> Self {
        Self {
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
            if v.can_be_number() {
                return Some(Value::Float(v.as_num()));
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
