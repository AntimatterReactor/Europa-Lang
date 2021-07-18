use std::rc::Rc;

use crate::environment::Environment;
use crate::error::{Error, ErrorType};
use crate::functions::{Call, Func, FuncType};
use crate::nodes::expr::Expr;
use crate::nodes::stmt::Stmt;
use crate::token::{TType, Token};
use crate::types::Type;

type IResult = Result<Type, Error>;
// type SResult = Result<(), Error>;

pub struct Interpreter {
    nodes: Vec<Stmt>,
    pub environ: Box<Environment>,
}

impl Interpreter {
    // static methods
    pub fn new(nodes: Vec<Stmt>, environ: Box<Environment>) -> Self {
        Self { nodes, environ }
    }

    pub fn stringify(value: Type) -> String {
        match value {
            Type::Array(v) => {
                let mut out = String::from('[');

                for (idx, i) in v.iter().enumerate() {
                    out += Self::stringify(i.clone()).as_str();

                    if idx < v.len() - 1 {
                        out += ", ";
                    }
                }

                out + "]"
            }
            Type::Nil => "nil".into(),
            Type::Float(n) => n.to_string(),
            Type::String(n) => n,
            Type::Bool(n) => n.to_string(),
            Type::Func(n) => n.to_string(),
            _ => "".into()
        }
    }

    pub fn init(&mut self) -> Result<(), Error> {
        self.environ.define(
            &String::from("println"),
            &Type::Func(FuncType::Native(Func::new(
                Rc::new(|_: &mut Interpreter, args: Vec<Type>| {
                    println!("{}", Self::stringify(args[0].clone()));
                    Ok(Type::Nil)
                }),
                1,
            ))),
        );

        for stmt in self.nodes.clone() {
            self.eval_stmt(&stmt.clone())?;
        }

        Ok(())
    }

    // eval
    fn eval_stmt(&mut self, node: &Stmt) -> IResult {
        match node {
            Stmt::ExprStmt(s) => self.eval_expr(s),
            Stmt::VarDecl(name, val) => {
                let val = self.eval_expr(&val)?;
                self.environ.define(&name, &val);
                Ok(Type::Nil)
            }
            Stmt::Block(stmts) => {
                self.eval_block(
                    Box::new(Environment::new_enclosing(Box::clone(&self.environ))),
                    stmts,
                )?;
                Ok(Type::Nil)
            }
            Stmt::IfStmt(cond, true_br, elif_brs, else_br) => {
                let cond_val = self.eval_expr(cond)?;

                if self.is_truthy(&cond_val) {
                    return Ok(self
                        .eval_block(
                            Box::new(Environment::new_enclosing(Box::clone(&self.environ))),
                            true_br,
                        )?
                        .unwrap());
                }
                if elif_brs.len() != 0 {
                    for (cond, elif_block) in elif_brs {
                        let cond_val = self.eval_expr(cond)?;
                        if self.is_truthy(&cond_val) {
                            return Ok(self
                                .eval_block(
                                    Box::new(Environment::new_enclosing(Box::clone(&self.environ))),
                                    elif_block,
                                )?
                                .unwrap());
                        }
                    }
                }
                if let Some(else_block) = else_br {
                    return Ok(self
                        .eval_block(
                            Box::new(Environment::new_enclosing(Box::clone(&self.environ))),
                            else_block,
                        )?
                        .unwrap());
                }

                Ok(Type::Nil)
            }
            Stmt::WhileStmt(cond, block) => {
                loop {
                    let cond = self.eval_expr(cond)?;
                    if !self.is_truthy(&cond) {
                        break;
                    }

                    let ret = self.eval_block(
                        Box::new(Environment::new_enclosing(Box::clone(&self.environ))),
                        block,
                    )?.unwrap();
                    
                    match ret {
                        Type::Break => break,
                        _ => ()
                    }
                }

                Ok(Type::Nil)
            },
            _ => Ok(Type::Nil)
        }
    }

    fn eval_expr(&mut self, node: &Expr) -> IResult {
        match node {
            Expr::Binary(left, tok, right) => {
                let lval = self.eval_expr(&left.as_ref())?;
                let rval = self.eval_expr(&right.as_ref())?;

                Ok(match tok.ttype {
                    TType::Plus => self.out(&lval.add(&rval), &tok)?,
                    TType::Minus => self.out(&lval.sub(&rval), &tok)?,
                    TType::Times => self.out(&lval.mult(&rval), &tok)?,
                    TType::Divide => self.out(&lval.div(&rval), &tok)?,
                    TType::Mod => self.out(&lval.modulo(&rval), &tok)?,

                    TType::EqEq => Type::Bool(lval == rval),
                    TType::NotEq => Type::Bool(lval != rval),

                    TType::Less => Type::Bool(lval < rval),
                    TType::Greater => Type::Bool(lval > rval),
                    TType::LessEq => Type::Bool(lval <= rval),
                    TType::GreaterEq => Type::Bool(lval >= rval),
                    _ => panic!(),
                })
            }
            Expr::Grouping(expr) => Ok(self.eval_expr(&expr.as_ref())?),
            Expr::Literal(val) => Ok(val.clone()),
            Expr::Unary(tok, right) => {
                let rval = self.eval_expr(&right.as_ref())?;

                match tok.ttype {
                    TType::Not => Ok(Type::Bool(self.is_truthy(&rval))),
                    _ => panic!(),
                }
            }
            Expr::Variable(v) => self.environ.get(v),
            Expr::Assign(k, v) => {
                let val = self.eval_expr(&v)?;
                self.environ.assign(k, &val)?;
                Ok(val)
            }
            Expr::Block(stmts) => Ok(self
                .eval_block(
                    Box::new(Environment::new_enclosing(Box::clone(&self.environ))),
                    stmts,
                )?
                .unwrap()),
            Expr::Logical(left, tok, right) => {
                let lval = self.eval_expr(left)?;

                if tok.ttype == TType::Or {
                    if self.is_truthy(&lval) {
                        return Ok(lval);
                    }
                } else {
                    if !self.is_truthy(&lval) {
                        return Ok(lval);
                    }
                }

                self.eval_expr(right)
            }
            Expr::Ternary(condition, true_br, else_br) => {
                let cond = self.eval_expr(condition)?;

                if self.is_truthy(&cond) {
                    return Ok(self.eval_expr(true_br)?);
                }

                Ok(self.eval_expr(else_br)?)
            }
            Expr::Call(func, tok, args) => {
                let callee = self.eval_expr(func)?;

                let mut params: Vec<Type> = Vec::new();
                for arg in args {
                    params.push(self.eval_expr(arg)?);
                }

                if let Type::Func(func) = callee {
                    if params.len() != func.arity() {
                        return Err(Error::new(
                            tok.lineinfo,
                            format!(
                                "Expected {} arguments, but got {}.",
                                func.arity(),
                                params.len()
                            )
                            .into(),
                            ErrorType::TypeError,
                        ));
                    }

                    func.call(self, params)?;
                } else {
                    return Err(Error::new(
                        tok.lineinfo,
                        "Only functions can be called.".into(),
                        ErrorType::TypeError,
                    ));
                }

                Ok(Type::Nil)
            }
        }
    }

    fn eval_block(
        &mut self,
        env: Box<Environment>,
        block: &Vec<Stmt>,
    ) -> Result<Option<Type>, Error> {
        self.environ = env.clone();
        let mut val = Type::Nil;

        for stmt in block {
            match stmt {
                Stmt::Break => {
                    val = Type::Break;
                    break;
                },
                Stmt::Continue => {
                    val = Type::Continue;
                    break;
                },
                _ => {
                    let ret = self.eval_stmt(stmt)?;
                    match ret {
                        Type::Break => {
                            val = Type::Break;
                            break;
                        },
                        Type::Continue => {
                            val = Type::Continue;
                            break;
                        },
                        _ => val = ret
                    }
                }
            }
        }

        self.environ = self.environ.parent.clone().unwrap();

        Ok(Some(val))
    }

    // util
    fn out(&self, val: &Result<Type, (String, ErrorType)>, tok: &Token) -> Result<Type, Error> {
        match val {
            Ok(r) => Ok(r.clone()),
            Err(t) => return Err(Error::new(tok.lineinfo, t.clone().0, t.clone().1)),
        }
    }

    fn is_truthy(&self, v: &Type) -> bool {
        match v {
            Type::Nil => false,
            Type::Bool(v) => *v,
            _ => true,
        }
    }
}
