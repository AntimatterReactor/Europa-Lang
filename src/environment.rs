use std::collections::HashMap;

use crate::{
    error::{Error, ErrorType},
    token::{TType, Token},
    types::Type,
};

#[derive(Debug, Clone)]
pub struct Environment {
    pub parent: Option<Box<Environment>>,
    values: HashMap<String, Type>,
}

impl Environment {
    pub fn new() -> Self {
        Self {
            parent: None,
            values: HashMap::new(),
        }
    }

    pub fn new_enclosing(parent: Box<Environment>) -> Self {
        Self {
            parent: Some(parent),
            values: HashMap::new(),
        }
    }

    pub fn get(&self, tok: &Token) -> Result<Type, Error> {
        match &tok.ttype {
            TType::Identifier(name) => {
                if self.values.contains_key(name) {
                    return Ok(self.values[name].clone());
                } else if let Some(parent) = &self.parent {
                    return parent.get(tok);
                }

                Err(Error::new(
                    tok.lineinfo,
                    format!("Undefined variable {}", name),
                    ErrorType::TypeError,
                ))
            }
            _ => panic!(),
        }
    }

    pub fn get_at(&self, distance: usize, name: &Token) -> Type {
        println!("{} {:#?} {:#?}", distance, self, self.ancestor(0));
        return self.ancestor(distance).get(name).unwrap();
    }

    fn ancestor(&self, distance: usize) -> Box<Self> {
        let mut env: Box<Self> = Box::new(self.clone());
        for _ in 0..distance {
            env = env.parent.unwrap();
        }

        env
    }

    pub fn define(&mut self, name: &String, val: &Type) {
        self.values.insert(name.clone(), val.clone());
    }

    pub fn assign(&mut self, name: &Token, val: &Type) -> Result<(), Error> {
        let ttype = name.ttype.clone();

        match ttype {
            TType::Identifier(k) => {
                if self.values.contains_key(&k) {
                    self.values.insert(k, val.clone());
                    return Ok(());
                } else if let Some(parent) = &mut self.parent {
                    return parent.assign(name, val);
                }

                Err(Error::new(
                    name.lineinfo,
                    format!("Undefined variable {}", k).into(),
                    ErrorType::TypeError,
                ))
            }
            _ => panic!(),
        }
    }

    pub fn assign_at(&mut self, distance: usize, name: &Token, val: &Type) -> Result<(), Error> {
        self.ancestor(distance).assign(name, val)
    }
}
