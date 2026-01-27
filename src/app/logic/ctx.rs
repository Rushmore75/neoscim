use std::{collections::HashMap, sync::RwLock};

use evalexpr::{error::EvalexprResultValue, *};

use crate::app::logic::{calc::Grid, cell::CellType};

pub struct CallbackContext<'a> {
    variables: &'a Grid,
    eval_breadcrumbs: RwLock<Vec<String>>,
    compute_cache: RwLock<HashMap<String, Value>>,
    functions: HashMap<String, Function<DefaultNumericTypes>>,

    /// True if builtin functions are disabled.
    without_builtin_functions: bool,
}

impl<'a> CallbackContext<'a> {
    fn expand_range(&self, range: &str) -> Option<Vec<&CellType>> {
        let v = range.split(':').collect::<Vec<&str>>();
        if v.len() == 2 {
            let start_col = v[0];
            let end_col = v[1];

            let as_index = |s: &str| {
                s.char_indices()
                    // .filter(|f| f.1 as u8 >= 97) // prevent sub with overflow errors
                    .map(|(idx, c)| ((c.to_ascii_lowercase() as usize).saturating_sub(97)) + (26 * idx))
                    .fold(0, |a, b| a + b)
            };

            let start_idx = as_index(start_col);
            let end_idx = as_index(end_col);

            let mut buf = Vec::new();

            for x in start_idx..=end_idx {
                for y in 0..=self.variables.max_y_at_x(x) {
                    if let Some(s) = self.variables.get_cell_raw(x, y) {
                        buf.push(s);
                    }
                }
            }
            return Some(buf);
        }
        None
    }

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
                        // there could be strings and whatnot in the range
                        if i.is_number() {
                            total += i.as_number()?;
                            count += 1.;
                        }
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
                        // there could be strings and whatnot in the range
                        if i.is_number() {
                            total += i.as_number()?;
                        }
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

        functions.insert(
            "xlookup".to_string(),
            Function::new(|arg| {
                let expected = vec![ValueType::String, ValueType::Tuple, ValueType::Tuple];
                if arg.is_tuple() {
                    let args = arg.as_tuple()?;

                    if args.len() == 3 {
                        let lookup_value = &args[0];
                        let lookup_array = &args[1];
                        let return_array = &args[2];

                        if lookup_array.is_tuple() && return_array.is_tuple() {
                            let mut found_at = None;
                            for (i, val) in lookup_array.as_tuple()?.iter().enumerate() {
                                if val == lookup_value {
                                    found_at = Some(i);
                                }
                            }
                            if let Some(i) = found_at {
                                if let Some(v) = return_array.as_tuple()?.get(i) {
                                    return Ok(v.clone());
                                }
                            }
                        }
                    }
                }
                Err(EvalexprError::TypeError {
                    expected,
                    actual: arg.clone(),
                })
            }),
        );

        functions
    }

    pub fn new(grid: &'a Grid) -> Self {
        Self {
            variables: grid,
            functions: Self::get_functions(),
            without_builtin_functions: false,
            eval_breadcrumbs: RwLock::new(Vec::new()),
            compute_cache: RwLock::new(HashMap::new()),
        }
    }
}

impl<'a> Context for CallbackContext<'a> {
    type NumericTypes = DefaultNumericTypes;

    fn get_value(&self, identifier: &str) -> Option<Value<Self::NumericTypes>> {
        
        // check cache
        if let Ok(cc) = self.compute_cache.read() {
            if let Some(hit) = cc.get(identifier) {
                return Some(hit.clone());
            }
        }

        if let Ok(mut trail) = self.eval_breadcrumbs.write() {
            let find = trail.iter().filter(|id| *id == identifier).count();
            if find > 0 {
                // recursion detected
                return None;
            } else {
                trail.push(identifier.to_owned(), );
            }
        }

        let pre_return = |v: Value| {
            if let Ok(mut cc) = self.compute_cache.write() {
                cc.insert(identifier.to_owned(), v.clone());
            }
            Some(v)
        };

        if let Some(v) = self.variables.get_cell(identifier) {
            match v {
                CellType::Number(n) => {
                    return pre_return(Value::Float(n.to_owned()));
                },
                CellType::String(s) => {
                    return pre_return(Value::String(s.to_owned()));
                },
                CellType::Equation(eq) => {
                    // remove the equals sign from the beginning, as that
                    // tries to set variables with our evaluation lib
                    match eval_with_context(&eq[1..], self) {
                        Ok(e) => {
                            return pre_return(e)
                        },
                        Err(e) => {
                            match e {
                                EvalexprError::VariableIdentifierNotFound(_err) => {
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
        } else {
            // identifier did not locate a cell, might be range
            if let Some(range) = self.expand_range(identifier) {
                let mut vals = Vec::new();
                for cell in range {
                    match cell {
                        CellType::Number(e) => vals.push(Value::Float(*e)),
                        CellType::String(s) => vals.push(Value::String(s.to_owned())),
                        CellType::Equation(eq) => {
                            match eval_with_context(&eq[1..], self) {
                                Ok(val) => vals.push(val),
                                Err(_err) => {
                                    // At this point we are getting an error because
                                    // recursion protection made this equation return
                                    // None. We now don't get any evaluation.
                                    return None
                                },
                            }
                        }
                    }
                }
                let v = Value::Tuple(vals);
                return Some(v);
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

/// DOES NOT EVALUATE EQUATIONS!!
///
/// This is used as a pseudo-context, just used for
/// learning all the variables in an expression.
#[derive(Debug)]
pub struct ExtractionContext {
    var_registry: RwLock<Vec<String>>,
    fn_registry: RwLock<Vec<String>>,
}

impl ExtractionContext {
    pub fn new() -> Self {
        Self {
            var_registry: RwLock::new(Vec::new()),
            fn_registry: RwLock::new(Vec::new()),
        }
    }
    pub fn dump_vars(&self) -> Vec<String> {
        if let Ok(r) = self.var_registry.read() { r.clone() } else { Vec::new() }
    }
    #[allow(dead_code)]
    pub fn dump_fns(&self) -> Vec<String> {
        if let Ok(r) = self.fn_registry.read() { r.clone() } else { Vec::new() }
    }
}

impl Context for ExtractionContext {
    type NumericTypes = DefaultNumericTypes;

    fn get_value(&self, identifier: &str) -> Option<Value<Self::NumericTypes>> {
        if let Ok(mut registry) = self.var_registry.write() {
            registry.push(identifier.to_owned());
        } else {
            panic!("The RwLock should always be write-able")
        }

        Some(Value::Int(1))
    }

    fn call_function(
        &self,
        identifier: &str,
        argument: &Value<Self::NumericTypes>,
    ) -> EvalexprResultValue<Self::NumericTypes> {
        let _ = argument;
        if let Ok(mut registry) = self.fn_registry.write() {
            registry.push(identifier.to_owned())
        } else {
            panic!("The RwLock should always be write-able")
        }
        // Ok(Value::Int(1))
        unimplemented!("Extracting function identifier not implemented yet")
    }

    fn are_builtin_functions_disabled(&self) -> bool {
        false
    }

    fn set_builtin_functions_disabled(&mut self, disabled: bool) -> EvalexprResult<(), Self::NumericTypes> {
        let _ = disabled;
        Ok(())
    }
}
