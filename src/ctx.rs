use std::{collections::HashMap, rc::Rc};

use evalexpr::{error::EvalexprResultValue, *};

use crate::Grid;

pub struct CallbackContext<'a, NumericTypes: EvalexprNumericTypes = DefaultNumericTypes> {
    variables: Rc<&'a Grid>,
    functions: HashMap<String, Function<NumericTypes>>,

    /// True if builtin functions are disabled.
    without_builtin_functions: bool,
}

impl<'a, NumericTypes: EvalexprNumericTypes> CallbackContext<'a, NumericTypes> {
    /// Constructs a `HashMapContext` with no mappings.
    pub fn new(grid: Rc<&'a Grid>) -> Self {
        Self {
            variables: grid,
            functions: Default::default(),
            without_builtin_functions: false,
        }
    }

    /// Removes all variables from the context.
    /// This allows to reuse the context without allocating a new HashMap.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use evalexpr::*;
    ///
    /// let mut context = HashMapContext::<DefaultNumericTypes>::new();
    /// context.set_value("abc".into(), "def".into()).unwrap();
    /// assert_eq!(context.get_value("abc"), Some(&("def".into())));
    /// context.clear_variables();
    /// assert_eq!(context.get_value("abc"), None);
    /// ```
    pub fn clear_variables(&mut self) {
        ()
    }

    /// Removes all functions from the context.
    /// This allows to reuse the context without allocating a new HashMap.
    pub fn clear_functions(&mut self) {
        self.functions.clear()
    }

    /// Removes all variables and functions from the context.
    /// This allows to reuse the context without allocating a new HashMap.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use evalexpr::*;
    ///
    /// let mut context = HashMapContext::<DefaultNumericTypes>::new();
    /// context.set_value("abc".into(), "def".into()).unwrap();
    /// assert_eq!(context.get_value("abc"), Some(&("def".into())));
    /// context.clear();
    /// assert_eq!(context.get_value("abc"), None);
    /// ```
    pub fn clear(&mut self) {
        self.clear_variables();
        self.clear_functions();
    }
}

impl<'a, NumericTypes: EvalexprNumericTypes> Context for CallbackContext<'a, NumericTypes> {
    type NumericTypes = NumericTypes;

    fn get_value(&self, identifier: &str) -> Option<&Value<Self::NumericTypes>> {
        return Some(4.);
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
    ) -> EvalexprResult<(), NumericTypes> {
        self.without_builtin_functions = disabled;
        Ok(())
    }
}

impl<'a, NumericTypes: EvalexprNumericTypes> ContextWithMutableVariables
    for CallbackContext<'a, NumericTypes>
{
    fn set_value(
        &mut self,
        identifier: String,
        value: Value<Self::NumericTypes>,
    ) -> EvalexprResult<(), NumericTypes> {
        Ok(())
    }

    fn remove_value(
        &mut self,
        identifier: &str,
    ) -> EvalexprResult<Option<Value<Self::NumericTypes>>, Self::NumericTypes> {
        // Removes a value from the `self.variables`, returning the value at the key if the key was previously in the map.
        // Ok(self.variables.remove(identifier))
        todo!();
    }
}

impl<'a, NumericTypes: EvalexprNumericTypes> ContextWithMutableFunctions
    for CallbackContext<'a, NumericTypes>
{
    fn set_function(
        &mut self,
        identifier: String,
        function: Function<NumericTypes>,
    ) -> EvalexprResult<(), Self::NumericTypes> {
        self.functions.insert(identifier, function);
        Ok(())
    }
}

impl<'b, NumericTypes: EvalexprNumericTypes> IterateVariablesContext for CallbackContext<'b, NumericTypes> {
    type VariableIterator<'a>
        = std::iter::Map<
        std::collections::hash_map::Iter<'a, String, Value<NumericTypes>>,
        fn((&String, &Value<NumericTypes>)) -> (String, Value<NumericTypes>),
    >
    where
        Self: 'a;
    type VariableNameIterator<'a>
        = std::iter::Cloned<std::collections::hash_map::Keys<'a, String, Value<NumericTypes>>>
    where
        Self: 'a;

    fn iter_variables(&self) -> Self::VariableIterator<'_> {
        todo!()
        // self.variables.iter().map(|(string, value)| (string.clone(), value.clone()))
    }

    fn iter_variable_names(&self) -> Self::VariableNameIterator<'_> {
        todo!()
        // self.variables.keys().cloned()
    }
}

